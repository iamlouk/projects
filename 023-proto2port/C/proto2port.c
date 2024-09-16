#include <arpa/inet.h>
#include <asm-generic/errno-base.h>
#include <assert.h>
#include <cson.h>
#include <errno.h>
#include <fcntl.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <regex.h>
#include <signal.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/epoll.h>
#include <sys/socket.h>
#include <unistd.h>

#include "./utils.h"

static volatile bool done = false;
static int epollfd = -1;
static const size_t MAX_EVENTS = 64;
static struct epoll_event e, events[64];

static void signal_handler(int sig) {
  if (sig == SIGINT && epollfd != -1) {
    close(epollfd);
    epollfd = -1;
  }
}

struct protocol_t {
  const char *name;
  enum { MATCH_FIRSTLINE } matchmode;
  regex_t regex;
  uint16_t dstport;
  struct protocol_t *next;
};

static struct protocol_t *protocols = NULL;

struct connection_state {
  int fds_client[2];
  int fds_srvice[2];
  struct protocol_t *protocol;
  struct connection_state *prev, *next;

  struct rbuffer *buf_incoming;
  struct rbuffer *buf_outgoing;
};

static struct connection_state *connections = NULL;

static struct connection_state *
new_connection(int client_fd_r, int client_fd_w) {
  struct connection_state *cs = calloc(1, sizeof(struct connection_state));
  cs->buf_incoming = rbuffer_create(2048);
  cs->buf_outgoing = rbuffer_create(2048);
  cs->fds_client[0] = client_fd_r;
  cs->fds_client[1] = client_fd_w;
  cs->fds_srvice[0] = -1;
  cs->fds_srvice[1] = -1;
  if (connections) {
    cs->next = connections;
    connections->prev = cs;
  }
  connections = cs;
  return cs;
}

static struct connection_state *
find_connection(int fd, unsigned in_or_out_fd) {
  for (struct connection_state *cs = connections;
       cs != NULL; cs = cs->next)
    if (cs->fds_client[in_or_out_fd] == fd ||
        cs->fds_srvice[in_or_out_fd] == fd)
      return cs;
  return NULL;
}

/* TODO: All of this function is missing cleanup in case of errors! */
static bool
dial_to_service(struct connection_state *cs) {
  struct sockaddr_in6 addr = {
    .sin6_family = AF_INET6,
    .sin6_port = htons(cs->protocol->dstport),
  };

  int fd1 = socket(AF_INET6, SOCK_STREAM, 0);
  if (fd1 < 0) {
    fprintf(stderr, "[P2P] error: %s\n", strerror(errno));
    return false;
  }

  if (connect(fd1, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
    fprintf(stderr, "[P2P] connection to %s failed: %s\n", cs->protocol->name, strerror(errno));
    return false;
  }

  int fd2 = dup(fd1);
  if (fd2 < 0) {
    fprintf(stderr, "[P2P] error: %s\n", strerror(errno));
    return false;
  }

  fcntl(fd1, F_SETFL, fcntl(fd1, F_GETFL, 0) | O_NONBLOCK);
  e.events = EPOLLIN | EPOLLRDHUP;
  e.data.fd = fd1;
  epoll_ctl(epollfd, EPOLL_CTL_ADD, fd1, &e);
  cs->fds_srvice[0] = fd1;

  fcntl(fd2, F_SETFL, fcntl(fd2, F_GETFL, 0) | O_NONBLOCK);
  e.events = EPOLLOUT | EPOLLRDHUP;
  e.data.fd = fd2;
  epoll_ctl(epollfd, EPOLL_CTL_ADD, fd2, &e);
  cs->fds_srvice[1] = fd2;

  return true;
}

static bool
handle_event(struct connection_state *cs, int fd) {
  /*
   * Move data around if possible:
   */
  ssize_t n = 0;
  if (fd == cs->fds_client[0] && (n = rbuffer_read_from_fd(cs->buf_incoming, fd)) <= 0) {
    if (errno != 0)
      fprintf(stderr, "read(client) failed: %s\n", strerror(errno));
    epoll_ctl(epollfd, EPOLL_CTL_DEL, fd, NULL);
    close(fd);
    cs->fds_client[0] = -1;
  }
  if (fd == cs->fds_srvice[0] && (n = rbuffer_read_from_fd(cs->buf_outgoing, fd)) <= 0) {
    if (errno != 0)
      fprintf(stderr, "read(srvice) failed: %s\n", strerror(errno));
    epoll_ctl(epollfd, EPOLL_CTL_DEL, fd, NULL);
    close(fd);
    cs->fds_srvice[0] = -1;
  }
  if (fd == cs->fds_client[1] && !rbuffer_is_empty(cs->buf_incoming) &&
      (n = rbuffer_write_to_fd(cs->buf_incoming, fd)) <= 0) {
    if (errno != 0 && errno != EPIPE)
      fprintf(stderr, "write(client) failed: %s\n", strerror(errno));
    epoll_ctl(epollfd, EPOLL_CTL_DEL, fd, NULL);
    close(fd);
    cs->fds_client[1] = -1;
  }
  if (fd == cs->fds_srvice[1] && !rbuffer_is_empty(cs->buf_outgoing) &&
      (n = rbuffer_write_to_fd(cs->buf_outgoing, fd)) <= 0) {
    if (errno != 0 && errno != EPIPE)
      fprintf(stderr, "write(srvice) failed: %s\n", strerror(errno));
    epoll_ctl(epollfd, EPOLL_CTL_DEL, fd, NULL);
    close(fd);
    cs->fds_srvice[1] = -1;
  }

  /*
   * If client_fd_r is closed/done and the incoming buffer is empty, close service_fd_w.
   * If service_fd_r is closed/done and the outgoing buffer is empty, close client_fd_w.
   */
  if (rbuffer_is_empty(cs->buf_incoming) && cs->fds_client[0] == -1 && cs->fds_srvice[1] != -1) {
    epoll_ctl(epollfd, EPOLL_CTL_DEL, cs->fds_srvice[1], NULL);
    close(cs->fds_srvice[1]);
    cs->fds_srvice[1] = -1;
  }
  if (rbuffer_is_empty(cs->buf_outgoing) && cs->fds_srvice[0] == -1 && cs->fds_client[1] != -1) {
    epoll_ctl(epollfd, EPOLL_CTL_DEL, cs->fds_client[1], NULL);
    close(cs->fds_client[1]);
    cs->fds_client[1] = -1;
  }

  if (!cs->protocol) {
    const char *line = NULL;
    n = rbuffer_get_line(cs->buf_incoming, &line);
    if (n < 0) {
      /* TODO: Give it X tries or so? */
      return false;
    }

    char buf[n + 1];
    memcpy(buf, line, n);
    buf[n] = '\0';

    assert(line != NULL);
    for (struct protocol_t *p = protocols; p != NULL; p = p->next) {
      if (!regexec(&p->regex, buf, 0, NULL, 0)) {
        cs->protocol = p;
        break;
      }
    }

    if (cs->protocol) {
      bool ok = dial_to_service(cs);
      assert("TODO..." && false);
      return true;
    }

    assert("TODO..." && false);
    return false;
  }

  assert("TODO..." && false);
  return true;
}

int main(int argc, const char *argv[]) {
  assert(sizeof(events) / sizeof(events[0]) == MAX_EVENTS);
  if (argc != 2)
    return fprintf(stderr, "usage: %s <config.json>\n", argv[0]), EXIT_FAILURE;

  // Setup and configuration (parsing):
  const char *freewhendone = NULL;
  struct cson *config_json = cson_parse_file(argv[1], &freewhendone);
  if (!config_json)
    return fprintf(stderr, "%s: I/O error: %s\n", argv[1], strerror(errno)),
           EXIT_FAILURE;

  if (config_json->type == CSON_ERROR)
    return fprintf(stderr, "%s(%d:%d): JSON parse error: %s\n", argv[1],
                   config_json->line, config_json->col, config_json->error),
           EXIT_FAILURE;

  int64_t port;
  char buffer[512];
  const char *fallback_response;
  const struct cson *field, *protocols_json;
  if (!cson_map_get_field(config_json, "port", &field) ||
      !cson_get_integer(field, &port))
    return fprintf(stderr, "%s: missing field: \"port\"\n", argv[1]),
           EXIT_FAILURE;
  if (!cson_map_get_field(config_json, "fallback-response", &field) ||
      !cson_get_string(field, &fallback_response))
    return fprintf(stderr, "%s: missing field: \"fallback-response\"\n",
                   argv[1]),
           EXIT_FAILURE;
  if (!cson_map_get_field(config_json, "protocols", &protocols_json) ||
      protocols_json->type != CSON_ARRAY)
    return fprintf(stderr, "%s: missing field: \"protocols\"\n", argv[1]),
           EXIT_FAILURE;

  int exit_code = EXIT_SUCCESS;
  size_t num_protocols = protocols_json->size;
  const struct cson *protocol_json = protocols_json->array;
  protocols = calloc(num_protocols, sizeof(struct protocol_t));
  for (size_t i = 0; i < protocols_json->size;
       i++, protocol_json = protocol_json->next) {
    struct protocol_t *p = &protocols[i];
    if (i != 0)
      protocols[i].next = p;
    int64_t dstport, err;
    const char *raw_regex, *mode;
    if (!cson_map_get_field(protocol_json, "name", &field) ||
        !cson_get_string(field, &p->name) ||
        !cson_map_get_field(protocol_json, "dstport", &field) ||
        !cson_get_integer(field, &dstport) ||
        !cson_map_get_field(protocol_json, "mode", &field) ||
        !cson_get_string(field, &mode) ||
        !cson_map_get_field(protocol_json, "match", &field) ||
        !cson_get_string(field, &raw_regex))
      return fprintf(stderr, "%s: protocol section mal-formatted.\n", argv[1]),
             EXIT_FAILURE;

    p->dstport = dstport;
    if ((err = regcomp(&p->regex, raw_regex,
                       REG_EXTENDED | REG_NOSUB | REG_NEWLINE)) != 0) {
      regerror(err, &p->regex, buffer, sizeof(buffer));
      fprintf(stderr, "invalid regular expression: '%s': %s\n", raw_regex,
              buffer);
      return EXIT_FAILURE;
    }

    if (strcmp(mode, "first-line") == 0)
      p->matchmode = MATCH_FIRSTLINE;
    else
      return fprintf(stderr, "unknown match mode: "), EXIT_FAILURE;

    fprintf(stderr, "[P2P]: <%s>\t(regex: \"%s\") -> %d\n", p->name, raw_regex,
            p->dstport);
  }

  // Setup of the TCP server socket that accepts connections:
  int listenfd = socket(AF_INET6, SOCK_STREAM, 0);
  struct sockaddr_in6 server_addr = {
      .sin6_family = AF_INET6,
      .sin6_port = htons(port),
  };
  const char *server_addr_txt = "::";
  inet_pton(AF_INET6, server_addr_txt, &server_addr.sin6_addr);
  if (listenfd == -1 ||
      bind(listenfd, (struct sockaddr *)&server_addr, sizeof(server_addr)) ==
          -1 ||
      listen(listenfd, 16) == -1 ||
      fcntl(listenfd, F_SETFL, fcntl(listenfd, F_GETFL, 0) | O_NONBLOCK) ==
          -1) {
    exit_code = EXIT_FAILURE;
    fprintf(stderr, "listen/bind to (%s):%d failed: %s\n",
            server_addr_txt, (uint16_t)port, strerror(errno));
    goto cleanup;
  }

  e.events = EPOLLIN;
  e.data.fd = listenfd;
  if ((epollfd = epoll_create1(0)) == -1 ||
      epoll_ctl(epollfd, EPOLL_CTL_ADD, listenfd, &e) == -1) {
    exit_code = EXIT_FAILURE;
    fprintf(stderr, "linux epoll setup failed: %s\n", strerror(errno));
    goto cleanup;
  }

  // Register a ^C handler:
  struct sigaction sigintact = { .sa_handler = &signal_handler };
  if (sigaction(SIGINT, &sigintact, NULL) == -1) {
    exit_code = EXIT_FAILURE;
    fprintf(stderr, "sigaction(SIGINT, ...) failed: %s\n", strerror(errno));
    goto cleanup;
  }

  fprintf(stderr, "[P2P]: Server started, listening on (%s):%d...\n",
          server_addr_txt, (uint16_t)port);

  for (size_t i = 0; !done; i++) {
    int nfds = epoll_wait(epollfd, events, MAX_EVENTS, -1);
    if (nfds == -1) {
      if (errno != EINTR || epollfd != -1) {
        exit_code = EXIT_FAILURE;
        fprintf(stderr, "linux epoll failed(%ld): %s\n", i, strerror(errno));
      }
      goto cleanup;
    }

    for (int n = 0; n < nfds; n++) {
      int fd = events[n].data.fd;
      /* Handle a new connection on the main listen socket... */
      if (fd == listenfd) {
        struct sockaddr_in6 client_addr;
        unsigned int addr_len = sizeof(client_addr);
        int nconnfd_r =
            accept(listenfd, (struct sockaddr *)&client_addr, &addr_len);
        int nconnfd_w;
        if (nconnfd_r == -1 || (nconnfd_w = dup(nconnfd_r)) == -1) {
          fprintf(stderr, "accept failed: %s\n", strerror(errno));
          continue;
        }

        fcntl(nconnfd_r, F_SETFL, fcntl(nconnfd_r, F_GETFL, 0) | O_NONBLOCK);
        e.events = EPOLLIN | EPOLLRDHUP;
        e.data.fd = nconnfd_r;
        if (epoll_ctl(epollfd, EPOLL_CTL_ADD, nconnfd_r, &e) == -1) {
          exit_code = EXIT_FAILURE;
          fprintf(stderr, "epoll_ctl failed: %s\n", strerror(errno));
          goto cleanup;
        }

        fcntl(nconnfd_w, F_SETFL, fcntl(nconnfd_w, F_GETFL, 0) | O_NONBLOCK);
        e.events = EPOLLOUT | EPOLLRDHUP;
        e.data.fd = nconnfd_w;
        if (epoll_ctl(epollfd, EPOLL_CTL_ADD, nconnfd_w, &e) == -1) {
          exit_code = EXIT_FAILURE;
          fprintf(stderr, "epoll_ctl failed: %s\n", strerror(errno));
          goto cleanup;
        }

        new_connection(nconnfd_r, nconnfd_w);
        continue;
      }

      struct connection_state *cs = NULL;
      if ((events[n].events & EPOLLIN) && !(events[n].events & EPOLLOUT))
        cs = find_connection(fd, 0);
      else if ((events[n].events & EPOLLOUT) && !(events[n].events & EPOLLIN))
        cs = find_connection(fd, 1);
      else
        assert("must use dedicated FDs for reads and writes" && false);

      handle_event(cs, fd);
    }
  }

cleanup:
  /* Shutdown and cleanup: */
  fprintf(stderr, "\n[P2P]: Shutting down... (connections will be cut!)\n");
  if (listenfd != -1)
    close(listenfd);
  for (size_t i = 0; i < num_protocols; i++)
    regfree(&protocols[i].regex);
  for (struct connection_state *conn = connections, *next = NULL;
       conn != NULL; conn = next) {
    next = conn->next;
    rbuffer_destroy(conn->buf_incoming);
    rbuffer_destroy(conn->buf_outgoing);
    free(conn);
  }
  free(protocols);
  cson_free(config_json);
  free((void *)freewhendone);
  fprintf(stderr, "[P2P]: Done\n");
  return exit_code;
}
