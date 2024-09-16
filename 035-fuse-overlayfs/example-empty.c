#include <errno.h>
#include <fuse/fuse.h>
#include <fuse/fuse_common.h>
#include <fuse/fuse_lowlevel.h>
#include <fuse/fuse_opt.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <time.h>

static struct options {
  bool show_help;
} options;

static const struct fuse_opt option_spec[] = {
    {"--help", offsetof(struct options, show_help), 1}, FUSE_OPT_END};

static int getattr(const char *path, struct stat *stbuf) {
  int ret = 0;
  memset(stbuf, 0, sizeof(struct stat));
  if (strcmp(path, "/") == 0) {
    stbuf->st_mode = S_IFDIR | 0755;
    stbuf->st_nlink = 2;
  } else {
    ret = -ENOENT;
  }
  return ret;
}

static int readdir(const char *path, void *buf, fuse_fill_dir_t filler,
                   off_t offset, struct fuse_file_info *fi) {
  (void)offset;
  (void)fi;
  if (strcmp(path, "/") != 0)
    return -ENOENT;

  filler(buf, ".", NULL, 0);
  filler(buf, "..", NULL, 0);
  return 0;
}

static const struct fuse_operations fs_ops = {.getattr = &getattr,
                                              .readdir = &readdir};

int main(int argc, const char *argv[]) {
  struct fuse_args args = FUSE_ARGS_INIT(argc, (char **)argv);
  if (fuse_opt_parse(&args, &options, option_spec, NULL) == -1)
    return EXIT_FAILURE;

  int ec = fuse_main(args.argc, args.argv, &fs_ops, NULL);
  fuse_opt_free_args(&args);
  return ec;
}
