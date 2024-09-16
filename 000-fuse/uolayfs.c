#include <assert.h>
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <fuse/fuse.h>
#include <fuse/fuse_common.h>
#include <fuse/fuse_lowlevel.h>
#include <fuse/fuse_opt.h>
#include <libgen.h>
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

/*
 * TODO: - By default, libfuse will multi-thread. If there will be a caching
 *         mechanism, then I need locking!
 * TODO: - A lot of error handling etc. is missing!
 *
 *
 *
 *
 *
 *
 */

#define MAX_PATH_DEPTH 16

#define IS_DIR(mode) (((mode) & S_IFMT) == S_IFDIR)
#define IS_REG(mode) (((mode) & S_IFMT) == S_IFREG)

static FILE *flog = NULL;
static int underlying_fd = -1;
static int overlaying_fd = -1;

struct uolayfs_deleted {
  size_t buf_size;
  size_t num_deleted;
  const char *buf;
  const char **filenames;
};

static struct options {
  const char *underlying_path;
  const char *overlaying_path;
  const char *log_filename;
  bool show_help;
} options;

static const struct fuse_opt option_spec[] = {
    {"--help", offsetof(struct options, show_help), 1},
    {"--underlying=%s", offsetof(struct options, underlying_path), 1},
    {"--overlayed=%s", offsetof(struct options, overlaying_path), 1},
    {"--logs=%s", offsetof(struct options, log_filename), 1},
    FUSE_OPT_END};

/* Read all of the file into a string buffer. */
static char *read_complete_file(int basefd, const char *dirname,
                                const char *basename, size_t *file_size) {
  char pathbuf[PATH_MAX];
  if (snprintf(&pathbuf[0], sizeof(pathbuf), "%s/%s", dirname, basename) < 0)
    return NULL;

  int fd = openat(basefd, pathbuf, O_RDONLY);
  if (fd < 0)
    return NULL;

  FILE *f = fdopen(fd, "r");
  fseek(f, 0, SEEK_END);
  size_t size = ftell(f);
  fseek(f, 0, SEEK_SET);
  char *contents = malloc(size + 1);
  if (fread(contents, size, 1, f) != 1) {
    free(contents);
    return NULL;
  }
  fclose(f);
  contents[size] = '\0';
  *file_size = size;
  return contents;
}

/* Create all directory levels needed. */
static bool create_intermediate_directories(const char *path) {
  size_t pathlen = strlen(path);
  bool success = true;
  for (size_t i = 0; i < pathlen; i++) {
    if (path[i] == '/' && path[i + 1] != '\0') {
      ((char *)path)[i] = '\0';
      if (mkdirat(overlaying_fd, path, 0755) != 0 && errno != EEXIST)
        success = false;
      ((char *)path)[i] = '/';
    }
  }
  return success;
}

/* Read a filename list into the uolayfs_deleted structure. */
static bool uolayfs_deleted_parse(const char *dirname, const char *basename,
                                  struct uolayfs_deleted *d) {
  d->buf = read_complete_file(overlaying_fd, dirname, basename, &d->buf_size);
  if (d->buf == NULL)
    return false;

  size_t num_lines = 1;
  for (size_t i = 0; i < d->buf_size; i++)
    if (d->buf[i] == '\n')
      num_lines += 1;

  char *saveptr = NULL;
  d->num_deleted = 0;
  d->filenames = malloc(num_lines * sizeof(const char *));
  while ((d->filenames[d->num_deleted] = strtok_r(
              d->num_deleted == 0 ? (char *)d->buf : NULL, "\n", &saveptr))) {
    assert(d->num_deleted == 0 || strcmp(d->filenames[d->num_deleted - 1],
                                         d->filenames[d->num_deleted]) < 0);
    d->num_deleted += 1;
  }
  return true;
}

/* Insert a filename into the uolayfs_deleted structure and then write it to the
 * specified destination. NOTE: Avoid re-opening the same file? */
static bool uolayfs_deleted_insert_and_write(struct uolayfs_deleted *d,
                                             const char *deleted,
                                             const char *dirname,
                                             const char *basename) {
  char pathbuf[PATH_MAX];
  snprintf(&pathbuf[0], sizeof(pathbuf), "%s/%s", dirname, basename);
  FILE *f = fdopen(
      openat(overlaying_fd, pathbuf, O_WRONLY | O_CREAT | O_TRUNC, 0755), "w");
  if (!f)
    return false;

  if (d->num_deleted == 0) {
    fwrite(deleted, strlen(deleted), 1, f);
    return fclose(f) == 0;
  }

  size_t i = 0;
  for (; i < d->num_deleted && strcmp(d->filenames[i], deleted) < 0; i++) {
    fwrite(d->filenames[i], strlen(d->filenames[i]), 1, f);
    fputs("\n", f);
  }
  fwrite(deleted, strlen(deleted), 1, f);
  fputs("\n", f);
  for (; i < d->num_deleted; i++) {
    fwrite(d->filenames[i], strlen(d->filenames[i]), 1, f);
    fputs("\n", f);
  }
  return fclose(f) == 0;
}

/* Make a relative path auto of raw useable for the ...at() libc functions. */
static const char *canonicalize_path(const char *raw, char buf[PATH_MAX]) {
  assert(raw[0] == '/' && "path not starting at root?");
  while (raw[0] == '/')
    raw = &raw[1];

  if (raw[0] == '\0') {
    buf[0] = '.';
    buf[1] = '\0';
    raw = &buf[0];
  }

  /* TODO: Remove trailing slashes, remove trailing '.' and '..'. */
  assert(raw[strlen(raw) - 1] != '/');
  return raw;
}

/* Return true if a file or directory exists. */
static bool exists(int dirfd, const char *path) {
  return faccessat(dirfd, path, F_OK, 0) == 0;
}

static int op_getattr(const char *path, struct stat *stbuf) {
  char pathbuf[PATH_MAX];
  path = canonicalize_path(path, &pathbuf[0]);
  errno = 0;
  if (fstatat(overlaying_fd, path, stbuf, 0) < 0 && errno == ENOENT) {
    errno = 0;
    fstatat(underlying_fd, path, stbuf, 0);
  }

  return -errno;
}

static const struct stat *stbuf_for_dirent(const struct dirent *de,
                                           struct stat *stbuf) {
  stbuf->st_ino = de->d_ino;
  stbuf->st_mode = DTTOIF(de->d_type);
  return stbuf;
}

/* Show files from the overlayed and underlayed version of a directory.
 * If a file exists in both, only show the overlayed directories entry.
 */
static const char UOLAYFS_DELETED_FILES[] = ".uolayfs-deleted";
static int op_readdir(const char *path, void *buf, fuse_fill_dir_t filler,
                      off_t offset, struct fuse_file_info *fi) {
  (void)offset;
  (void)fi;
  char pathbuf[PATH_MAX];
  path = canonicalize_path(path, &pathbuf[0]);

  struct dirent **odirents = NULL, **udirents = NULL;
  int on = scandirat(overlaying_fd, path, &odirents, NULL, &alphasort);
  size_t odirents_len = on;
  if (on < 0) {
    if (errno != ENOENT)
      return -errno;

    odirents_len = 0;
  }

  struct uolayfs_deleted deleted_files = {0};
  for (size_t i = 0; i < odirents_len; i++)
    if (strcmp(UOLAYFS_DELETED_FILES, odirents[i]->d_name) == 0)
      uolayfs_deleted_parse(path, UOLAYFS_DELETED_FILES, &deleted_files);

  int un = scandirat(underlying_fd, path, &udirents, NULL, &alphasort);
  size_t udirents_len = un;
  if (un < 0) {
    if (on == 0) {
      for (size_t i = 0; i < odirents_len; i++)
        free(odirents[i]);
      free(odirents);
      return -errno;
    }
    udirents_len = 0;
  }

  struct stat stbuf;
  memset((void *)&stbuf, 0, sizeof(stbuf));

  size_t opos = 0, upos = 0;
  const char **delpos = deleted_files.filenames;
  while (opos < odirents_len && upos < udirents_len) {
    struct dirent *od = odirents[opos];
    if (strcmp(od->d_name, UOLAYFS_DELETED_FILES) == 0) {
      free(od);
      opos += 1;
      continue;
    }

    struct dirent *ud = udirents[upos];
    int cmp = strcmp(od->d_name, ud->d_name);
    /* If the file exits in both directories, only show the overlayed one. */
    if (cmp == 0) {
      filler(buf, od->d_name, stbuf_for_dirent(od, &stbuf), 0);
      free(od);
      free(ud);
      opos += 1;
      upos += 1;
      continue;
    }

    if (cmp < 0) {
      filler(buf, od->d_name, stbuf_for_dirent(od, &stbuf), 0);
      free(od);
      opos += 1;
      continue;
    }

    /* If the file is deleted, skip over it. */
    if (delpos != NULL && *delpos != NULL && strcmp(*delpos, ud->d_name) == 0)
      delpos += 1;
    else
      filler(buf, ud->d_name, stbuf_for_dirent(ud, &stbuf), 0);
    free(ud);
    upos += 1;
  }

  while (opos < odirents_len) {
    filler(buf, odirents[opos]->d_name,
           stbuf_for_dirent(odirents[opos], &stbuf), 0);
    free(odirents[opos]);
    opos += 1;
  }

  while (upos < udirents_len) {
    if (delpos != NULL && *delpos != NULL &&
        strcmp(*delpos, udirents[upos]->d_name) == 0)
      delpos += 1;
    else
      filler(buf, udirents[upos]->d_name,
             stbuf_for_dirent(udirents[upos], &stbuf), 0);
    free(udirents[upos]);
    upos += 1;
  }

  free((void *)deleted_files.filenames);
  free((void *)deleted_files.buf);
  free(odirents);
  free(udirents);
  return -errno;
}

static int helper_remove(const char *path, int flags) {
  char pathbuf[PATH_MAX];
  path = canonicalize_path(path, &pathbuf[0]);
  bool exists_in_ufs = exists(underlying_fd, path);
  errno = 0;
  if (unlinkat(overlaying_fd, path, flags) == 0) {
    if (!exists_in_ufs)
      return 0;
  }

  /* This is probably wrong/not actually proper. There are many
   * reasons these two error codes could be returned. */
  if ((errno != ENOENT && errno != ENOTDIR) || !exists_in_ufs)
    return -errno;

  /* TODO: This is untested! */
  char dirnamebuf[PATH_MAX];
  strcpy(&dirnamebuf[0], path);
  const char *dname = dirname(&dirnamebuf[0]);
  struct uolayfs_deleted deleted_files = {0};
  bool ok = uolayfs_deleted_parse(path, UOLAYFS_DELETED_FILES, &deleted_files);
  if (!ok) {
    assert(errno == ENOENT);
    create_intermediate_directories(path);
  }

  ok = uolayfs_deleted_insert_and_write(&deleted_files, basename((char *)path),
                                        dname, UOLAYFS_DELETED_FILES);
  assert(ok);
  return 0;
}

static int op_unlink(const char *path) { return helper_remove(path, 0); }

static int op_rmdir(const char *path) {
  return helper_remove(path, AT_REMOVEDIR);
}

static const struct fuse_operations fs_ops = {.getattr = &op_getattr,
                                              .readdir = &op_readdir,
                                              .unlink = &op_unlink,
                                              .rmdir = &op_rmdir};

int main(int argc, const char *argv[]) {
  struct fuse_args args = FUSE_ARGS_INIT(argc, (char **)argv);
  options.log_filename = strdup("/dev/stderr");
  if (fuse_opt_parse(&args, &options, option_spec, NULL) == -1)
    return EXIT_FAILURE;

  flog = fopen(options.log_filename, "w");
  if (!flog) {
    fprintf(stderr, "uolayfs: cannot open %s: %s\n", options.log_filename,
            strerror(errno));
    return EXIT_FAILURE;
  }

  if (!options.underlying_path || !options.overlaying_path) {
    fprintf(stderr,
            "uolayfs: required option: --underlying=... and --overlayed=...\n");
    return EXIT_FAILURE;
  }

  if ((underlying_fd = open(options.underlying_path, O_DIRECTORY | O_PATH)) <
          0 ||
      (overlaying_fd = open(options.overlaying_path, O_DIRECTORY | O_PATH)) <
          0) {
    fprintf(stderr, "uolayfs: failed to open '%s' or '%s': %s\n",
            options.underlying_path, options.overlaying_path, strerror(errno));
    return EXIT_FAILURE;
  }

  fprintf(flog,
          "uolayfs: underlying='%s'->%d, overlayed='%s'->%d, errlog='%s'\n",
          options.underlying_path, underlying_fd, options.overlaying_path,
          overlaying_fd, options.log_filename);
  int ec = fuse_main(args.argc, args.argv, &fs_ops, NULL);
  fuse_opt_free_args(&args);
  errno = 0;
  close(underlying_fd);
  close(overlaying_fd);
  fprintf(flog, "uolayfs: done (errno=%s)!\n", strerror(errno));
  fclose(flog);
  return ec;
}
