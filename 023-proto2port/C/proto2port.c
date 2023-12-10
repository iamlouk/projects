#include <arpa/inet.h>
#include <assert.h>
#include <cson.h>
#include <errno.h>
#include <fcntl.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <regex.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/epoll.h>
#include <sys/socket.h>
#include <unistd.h>

static volatile bool done = false;
static int epollfd = -1;
static const size_t MAX_EVENTS = 16;
static struct epoll_event e, events[16];

struct protocol_t {
  const char *name;
  enum { MATCH_FIRSTLINE } matchmode;
  regex_t regex;
  uint16_t dstport;
  struct protocol_t *next;
};

static struct protocol_t *protocols = NULL;

struct connection_state {
  int fd1, fd2; /* fd1: the socket that connected to this server,
                   fd2: the socket from this server to the new port. */
  struct protocol_t *protocol;
  struct connection_state *prev, *next;
  size_t buffer_len;
  size_t buffer_cap;
  char *buffer;
};

static struct connection_state *connections = NULL;
static struct connection_state *connections_by_fd_hash[128] = {NULL};
static inline size_t hash_connection_fd(unsigned fd) {
  return (fd ^ (fd >> 10)) %
         (sizeof(connections_by_fd_hash) / sizeof(connections_by_fd_hash[0]));
}

static struct connection_state *
new_connection(int fd, struct sockaddr *client_addr, size_t addr_len) {
  (void)client_addr;
  (void)addr_len;
  struct connection_state *cs = calloc(1, sizeof(struct connection_state));
  cs->fd1 = fd;
  cs->fd2 = -1;
  cs->next = connections;
  cs->protocol = NULL;
  cs->buffer_len = 0;
  cs->buffer_cap = 1 << 12;
  cs->buffer = calloc(cs->buffer_cap, sizeof(char));
  if (connections) {
    assert(connections->prev == NULL);
    connections->prev = cs;
  }

  connections = cs;
  connections_by_fd_hash[hash_connection_fd(fd)] = cs;
  return cs;
}

static struct connection_state *find_connection(int fd) {
  size_t hash = hash_connection_fd(fd);
  struct connection_state *hashed = connections_by_fd_hash[hash];
  if (hashed && (hashed->fd1 == fd || hashed->fd2 == fd))
    return hashed;

  for (struct connection_state *cs = connections; cs != NULL; cs = cs->next)
    if (cs->fd1 == fd || cs->fd2 == fd)
      return (connections_by_fd_hash[hash] = cs);

  return NULL;
}

/* TODO: What if there is still stuff to write?! */
#if 0
static void terminate_connection(struct connection_state *cs) {
  connections_by_fd_hash[hash_connection_fd(cs->connfd)] = NULL;
  connections_by_fd_hash[hash_connection_fd(cs->pipefds[0])] = NULL;
  connections_by_fd_hash[hash_connection_fd(cs->pipefds[1])] = NULL;
  if (cs == connections) {
    assert(cs->prev == NULL);
    connections = cs->next;
    return;
  }

  epoll_ctl(epollfd, EPOLL_CTL_DEL, cs->connfd, NULL);
  close(cs->connfd);
  epoll_ctl(epollfd, EPOLL_CTL_DEL, cs->pipefds[0], NULL);
  epoll_ctl(epollfd, EPOLL_CTL_DEL, cs->pipefds[1], NULL);
  close(cs->pipefds[0]);
  close(cs->pipefds[1]);

  assert(cs->prev != NULL);
  cs->prev->next = cs->next;
  cs->next->prev = cs->prev;
  free(cs);
}
#endif

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
  fprintf(stderr, "[P2P]: TCP port: %ld\n", port);
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

    fprintf(stderr, "[P2P]: '%s' (regex: '%s') -> %d\n", p->name, raw_regex,
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
    fprintf(stderr, "listen/bind to (%s):%d failed: %s\n", server_addr_txt,
            (uint16_t)port, strerror(errno));
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

  for (size_t i = 0; !done; i++) {
    int nfds = epoll_wait(epollfd, events, MAX_EVENTS, -1);
    if (nfds == -1) {
      exit_code = EXIT_FAILURE;
      fprintf(stderr, "linux epoll failed(%ld): %s\n", i, strerror(errno));
      goto cleanup;
    }

    for (int n = 0; n < nfds; n++) {
      int fd = events[n].data.fd;
      if (fd == listenfd) {
        struct sockaddr_in6 client_addr;
        unsigned int addr_len = sizeof(client_addr);
        int nconnfd =
            accept(listenfd, (struct sockaddr *)&client_addr, &addr_len);
        if (nconnfd == -1) {
          fprintf(stderr, "accept failed: %s\n", strerror(errno));
          continue;
        }

        fcntl(nconnfd, F_SETFL, fcntl(nconnfd, F_GETFL, 0) | O_NONBLOCK);
        e.events = EPOLLIN | EPOLLOUT | EPOLLRDHUP;
        e.data.fd = nconnfd;
        if (epoll_ctl(epollfd, EPOLL_CTL_ADD, nconnfd, &e) == -1) {
          exit_code = EXIT_FAILURE;
          fprintf(stderr, "epoll_ctl failed: %s\n", strerror(errno));
          goto cleanup;
        }

        new_connection(nconnfd, (struct sockaddr *)&client_addr, addr_len);
        continue;
      }

      struct connection_state *cs = find_connection(fd);
      assert(cs && "no connection found for event FD");

      /* TODO/FIXME: Handle connections...! */
    }
  }

cleanup:
  /* Shutdown and cleanup: */
  if (listenfd != -1)
    close(listenfd);
  for (size_t i = 0; i < num_protocols; i++)
    regfree(&protocols[i].regex);
  free(protocols);
  cson_free(config_json);
  free((void *)freewhendone);
  return exit_code;
}
