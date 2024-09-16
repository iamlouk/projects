#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <fuse/fuse.h>
#include <fuse/fuse_common.h>
#include <fuse/fuse_lowlevel.h>
#include <fuse/fuse_opt.h>
#include <linux/limits.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <time.h>
#include <unistd.h>

#ifndef O_PATH
#define O_PATH 010000000
#endif

#define MAX_PATH_DEPTH 16

#define IS_DIR(mode) (((mode) & S_IFMT) == S_IFDIR)
#define IS_REG(mode) (((mode) & S_IFMT) == S_IFREG)

static FILE *flog;
static int rootfd;
static uint64_t ino_max;

static struct options {
  const char *underlying_root;
  const char *log_filename;
  bool show_help;
} options;

static const struct fuse_opt option_spec[] = {
    {"--help", offsetof(struct options, show_help), 1},
    {"--underlying=%s", offsetof(struct options, underlying_root), 1},
    FUSE_OPT_END};

struct path_t {
  size_t n;
  const char *segs[MAX_PATH_DEPTH];
  const char raw[PATH_MAX];
};

static void segment_path(const char *raw, struct path_t *p) {
  assert(raw[0] == '/');
  strcpy((char *)&p->raw[0], raw);
  char *saveptr = NULL;
  raw += 1;
  p->n = 0;
  for (size_t i = 0; true; i++) {
    assert(i < MAX_PATH_DEPTH);
    char *tok = strtok_r(i == 0 ? (char *)p->raw : NULL, "/", &saveptr);
    if (!tok)
      break;
    p->segs[i] = tok;
    p->n += 1;
  }
  errno = 0;
}

struct ofs_file {
  uint64_t ino;
  mode_t mode;
  uid_t uid;
  gid_t gid;
  uint32_t name_len;
  const char *name;

  bool has_underlying;
  int underlying_fd;
  struct stat underlying_stats;

  struct ofs_file *parent;
  union {
    struct {
      size_t size;
      size_t capacity;
      uint8_t *data;
    } file;
    struct {
      size_t size;
      size_t capacity;
      struct ofs_file **files;
    } dir;
  };
};

/* Return the index or a pot. insert position of a file named name in dir. */
static size_t ofs_file_find_pos(struct ofs_file *dir, const char *name,
                                bool *found) {
  assert(IS_DIR(dir->mode));
  *found = false;
  size_t size = dir->dir.size;
  for (size_t i = 0; i < size; i++) {
    int cmp = strcmp(name, dir->dir.files[i]->name);
    if (cmp == 0) {
      *found = true;
      return i;
    } else if (cmp > 0) {
      return i;
    }
  }
  return size;
}

/* Return the ofs_file at this path. */
static struct ofs_file *ofs_file_find(struct ofs_file *dir,
                                      const struct path_t *path) {
  assert(IS_DIR(dir->mode));
  for (size_t depth = 0; depth < path->n; depth++) {
    bool exists;
    size_t pos = ofs_file_find_pos(dir, path->segs[depth], &exists);
    if (!exists) {
      errno = ENOENT;
      return NULL;
    }

    dir = dir->dir.files[pos];
    if (!IS_DIR(dir->mode)) {
      errno = ENOTDIR;
      return NULL;
    }
  }
  return dir;
}

/* Create a file or directcory, automatically adding it to dir if not NULL. */
static struct ofs_file *ofs_file_init(struct ofs_file *dir, bool is_file,
                                      const char *name) {
  size_t dir_pos = -1ul;
  if (dir) {
    bool exists = false;
    dir_pos = ofs_file_find_pos(dir, name, &exists);
    if (exists) {
      errno = EEXIST;
      return NULL;
    }
  }

  struct ofs_file *f = calloc(1, sizeof(struct ofs_file));
  assert(f);
  f->ino = (ino_max += 1);
  f->name_len = strlen(name);
  f->name = strdup(name);
  f->mode = S_IRUSR | S_IWUSR | S_IRGRP | S_IROTH;
  f->mode |= is_file ? (S_IFREG) : (S_IFDIR | S_IXUSR);
  f->uid = dir ? dir->uid : getuid();
  f->gid = dir ? dir->gid : getgid();
  f->parent = dir;
  if (dir) {
    assert(IS_DIR(dir->mode));
    /* Sorted insert: */
    size_t cap = dir->dir.capacity;
    if (dir->dir.size + 1 >= cap) {
      cap = dir->dir.capacity = (cap == 0 ? 8 : cap * 2);
      dir->dir.files = realloc(dir->dir.files, dir->dir.capacity);
      assert(dir->dir.files);
    }

    struct ofs_file **files = dir->dir.files;
    memmove((void *)files[dir_pos + 1], (void *)files[dir_pos],
            (dir->dir.size - dir_pos) * sizeof(struct ofs_file *));
    dir->dir.size += 1;
    files[dir_pos] = f;
  }
  fprintf(flog, "uoverlayfs: new file! ino=%d, parent=%d, name='%s'\n",
          (int)f->ino, dir ? (int)dir->ino : -1, name);
  return f;
}

static struct ofs_file *overlay_root;

static struct stat *fill_statbuf(const struct ofs_file *f, struct stat *stbuf) {
  if (f == NULL)
    return NULL;

  stbuf->st_mode = f->mode;
  stbuf->st_nlink = IS_DIR(f->mode) ? 2 : 1;
  stbuf->st_size = IS_DIR(f->mode) ? f->dir.size : f->file.size;
  stbuf->st_uid = f->uid;
  stbuf->st_gid = f->gid;
  stbuf->st_ino = f->ino;
  return stbuf;
}

static int op_getattr(const char *path, struct stat *stbuf) {
  struct path_t p;
  segment_path(path, &p);
  struct ofs_file *f = ofs_file_find(overlay_root, &p);
  if (!f) {
    errno = 0;
    (void)fstatat(rootfd, path, stbuf, 0x0);
    return -errno;
  }

  memset(stbuf, 0, sizeof(struct stat));
  fill_statbuf(f, stbuf);
  return 0;
}

static int op_readdir(const char *path, void *buf, fuse_fill_dir_t filler,
                      off_t offset, struct fuse_file_info *fi) {
  (void)offset; /* Can be ignored if fillfn is always called with zero. */
  (void)fi;
  struct path_t p;
  segment_path(path, &p);
  struct ofs_file *d = ofs_file_find(overlay_root, &p);
  /* TODO: Check underlying FS. */
  if (!d)
    return -ENOENT;
  if (!IS_DIR(d->mode))
    return -ENOTDIR;

  struct stat stbuf = {0};
  filler(buf, ".", fill_statbuf(d, &stbuf), 0);
  filler(buf, "..", fill_statbuf(d->parent, &stbuf), 0);
  for (size_t i = 0, n = d->dir.size; i < n; i++)
    filler(buf, d->dir.files[i]->name, fill_statbuf(d->dir.files[i], &stbuf),
           0);
  return 0;
}

static int op_mkdir(const char *path, mode_t mode) {
  (void)mode; /* Permissions etc. are ignored for now. */
  struct path_t p;
  segment_path(path, &p);
  assert(p.n > 0);
  const char *name = p.segs[--p.n];
  struct ofs_file *d = ofs_file_find(overlay_root, &p);
  if (!d)
    return -ENOENT;
  if (!IS_DIR(d->mode))
    return -ENOTDIR;

  struct ofs_file *f = ofs_file_init(d, false, name);
  if (!f)
    return -errno;

  return 0;
}

static const struct fuse_operations fs_ops = {
    .getattr = &op_getattr, .readdir = &op_readdir, .mkdir = &op_mkdir};

int main(int argc, const char *argv[]) {
  struct fuse_args args = FUSE_ARGS_INIT(argc, (char **)argv);
  options.log_filename = strdup("/dev/stderr");
  if (fuse_opt_parse(&args, &options, option_spec, NULL) == -1)
    return EXIT_FAILURE;

  flog = fopen(options.log_filename, "w");
  if (!flog) {
    fprintf(stderr, "cannot open %s: %s\n", options.log_filename,
            strerror(errno));
    return EXIT_FAILURE;
  }

  if (!options.underlying_root) {
    fprintf(stderr, "required option: --underlying=...\n");
    return EXIT_FAILURE;
  }

  rootfd = open(options.underlying_root, O_DIRECTORY);
  if (rootfd < 0) {
    fprintf(flog, "cannot open %s: %s\n", options.underlying_root,
            strerror(errno));
    return EXIT_FAILURE;
  }

  overlay_root = ofs_file_init(NULL, false, "");
  overlay_root->has_underlying = true;
  overlay_root->underlying_fd = rootfd;

  fprintf(flog, "uoverlayfs: uroot='%s', errlog='%s'\n",
          options.underlying_root, options.log_filename);
  int ec = fuse_main(args.argc, args.argv, &fs_ops, NULL);
  if (close(rootfd) < 0) {
    fprintf(flog, "cannot close %s: %s\n", options.underlying_root,
            strerror(errno));
    return EXIT_FAILURE;
  }

  fuse_opt_free_args(&args);
  /* TODO: Close all the underlying FDs in root sub-trees! */
  close(overlay_root->underlying_fd);
  fprintf(flog, "uoverlayfs: done!\n");
  fclose(flog);
  return ec;
}
