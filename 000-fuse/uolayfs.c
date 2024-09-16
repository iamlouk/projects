#include <assert.h>
#include <dirent.h>
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

#define MAX_PATH_DEPTH 16

#define IS_DIR(mode) (((mode) & S_IFMT) == S_IFDIR)
#define IS_REG(mode) (((mode) & S_IFMT) == S_IFREG)

static FILE *flog = NULL;
static int underlying_fd = -1;
static int overlayed_fd = -1;

static struct options {
  const char *underlying_path;
  const char *overlayed_path;
  const char *log_filename;
  bool show_help;
} options;

static const struct fuse_opt option_spec[] = {
    {"--help", offsetof(struct options, show_help), 1},
    {"--underlying=%s", offsetof(struct options, underlying_path), 1},
    {"--overlayed=%s", offsetof(struct options, overlayed_path), 1},
    {"--logs=%s", offsetof(struct options, log_filename), 1},
    FUSE_OPT_END};

/* Make a relative path auto of raw useable for the ...at() libc functions. */
static const char *canonicalize_path(const char *raw, char buf[PATH_MAX]) {
  assert(raw[0] == '/' && "path not starting at root?");
  while (raw[0] == '/')
    raw = &raw[1];

  if (raw[0] == '\0') {
    buf[0] = '.';
    buf[1] = '\0';
    return &buf[0];
  }

  return raw;
}

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
  if (fread(contents, size, 1, f) != size) {
    free(contents);
    return NULL;
  }
  fclose(f);
  contents[size] = '\0';
  *file_size = size;
  return contents;
}

static int op_getattr(const char *path, struct stat *stbuf) {
  char pathbuf[PATH_MAX];
  path = canonicalize_path(path, &pathbuf[0]);
  errno = 0;
  if (fstatat(overlayed_fd, path, stbuf, 0) < 0 && errno == ENOENT) {
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
 * FIXME: The deleted files stuff does not work!
 */
static const char UOLAYFS_DELETED_FILES[] = ".uolayfs-deleted";
static int op_readdir(const char *path, void *buf, fuse_fill_dir_t filler,
                      off_t offset, struct fuse_file_info *fi) {
  (void)offset;
  (void)fi;
  char pathbuf[PATH_MAX];
  path = canonicalize_path(path, &pathbuf[0]);

  struct dirent **odirents = NULL, **udirents = NULL;
  int on = scandirat(overlayed_fd, path, &odirents, NULL, &alphasort);
  size_t odirents_len = on;
  if (on < 0) {
    if (errno != ENOENT)
      return -errno;

    odirents_len = 0;
  }

  char *deleted_files_buf = NULL;
  size_t deleted_files_len = 0;
  char **deleted_files = NULL;
  size_t num_deleted_files = 0;
  for (size_t i = 0; i < odirents_len; i++)
    if (strcmp(UOLAYFS_DELETED_FILES, odirents[i]->d_name) == 0 &&
        (deleted_files_buf = read_complete_file(
             overlayed_fd, path, UOLAYFS_DELETED_FILES, &deleted_files_len))) {
      size_t max_lines = 1;
      for (size_t i = 0; i < deleted_files_len; i++)
        if (deleted_files_buf[i] == '\n')
          max_lines += 1;

      char *saveptr = NULL;
      deleted_files = calloc(max_lines, sizeof(char *));
      while ((deleted_files[num_deleted_files] =
                  strtok_r(num_deleted_files == 0 ? deleted_files_buf : NULL,
                           "\n", &saveptr)))
        num_deleted_files += 1;
    }

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
  const char **delpos = (const char **)deleted_files;
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
    if (delpos != NULL && *delpos != NULL &&
        strcmp(*delpos, udirents[upos]->d_name) == 0)
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

  free(deleted_files_buf);
  free(deleted_files);
  free(odirents);
  free(udirents);
  return -errno;
}

static const struct fuse_operations fs_ops = {.getattr = &op_getattr,
                                              .readdir = &op_readdir};

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

  if (!options.underlying_path || !options.overlayed_path) {
    fprintf(stderr,
            "uolayfs: required option: --underlying=... and --overlayed=...\n");
    return EXIT_FAILURE;
  }

  if ((underlying_fd = open(options.underlying_path, O_DIRECTORY | O_PATH)) <
          0 ||
      (overlayed_fd = open(options.overlayed_path, O_DIRECTORY | O_PATH)) < 0) {
    fprintf(stderr, "uolayfs: failed to open '%s' or '%s': %s\n",
            options.underlying_path, options.overlayed_path, strerror(errno));
    return EXIT_FAILURE;
  }

  fprintf(flog,
          "uolayfs: underlying='%s'->%d, overlayed='%s'->%d, errlog='%s'\n",
          options.underlying_path, underlying_fd, options.overlayed_path,
          overlayed_fd, options.log_filename);
  int ec = fuse_main(args.argc, args.argv, &fs_ops, NULL);
  fuse_opt_free_args(&args);
  errno = 0;
  close(underlying_fd);
  close(overlayed_fd);
  fprintf(flog, "uolayfs: done (errno=%s)!\n", strerror(errno));
  fclose(flog);
  return ec;
}
