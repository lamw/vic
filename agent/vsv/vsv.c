#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

#include "vmci_sockets.h"

/*
 * vsv - vsocket verification.
 * Program to verify vsocket connection between server() running on ESX host,
 * and client() running on ESX guest.
 */
static int default_port = 15000;
#define VMADDR_CID_HOST 2

int server() {
   struct sockaddr_vm addr;
   socklen_t len = sizeof addr;
   int af, sock, fd, c;
   uint32_t vmid;

   if ((af = VMCISock_GetAFValueFd(&fd)) == -1) {
       perror("VMCISock_GetAFValueFd");
       return -1;
   }

   if ((sock = socket(af, SOCK_STREAM, 0)) == -1) {
      perror("socket");
      return -1;
   }

   memset(&addr, 0, sizeof addr);
   addr.svm_family = af;
   addr.svm_cid = VMADDR_CID_ANY;
   addr.svm_port = default_port;

   if (bind(sock, (const struct sockaddr *)&addr, sizeof addr) == -1) {
      perror("bind");
      close(sock);
      return -1;
   }

   memset(&addr, 0, sizeof addr);

   if (getsockname(sock, (struct sockaddr *)&addr, &len) == -1) {
      perror("getsockname");
      close(sock);
      return -1;
   }

   if (listen(sock, 1) == -1) {
      perror("listen");
      return -1;
   }

   len = sizeof addr;
   if ((c = accept(sock, (struct sockaddr *)&addr, &len)) == -1) {
      perror("Failed to accept connection");
      return -1;
   }

   len = sizeof(vmid);
   if (getsockopt(c, af, SO_VMCI_PEER_HOST_VM_ID, &vmid, &len) == -1) {
      perror("getsockopt SO_VMCI_PEER_HOST_VM_ID");
      return -1;
   }

   printf("vmid=%d\n", vmid);

   memset(&addr, 0, sizeof addr);
   len = sizeof(addr);
   if (getpeername(c, (struct sockaddr *)&addr, &len) == -1) {
      perror("getpeername");
      return -1;
   }

   char buf[5];
   if ((len = recv(c, &buf, sizeof buf, 0)) == -1) {
       perror("recv");
       close(c);
       return -1;
   }

   if (strncmp(buf, "ping", 4) == 0) {
       memcpy(&buf, "pong\0", sizeof buf);
   }

   if (send(c, &buf, sizeof buf, 0) != sizeof buf) {
       perror("send");
       return -1;
   }

   return 0;
}

int client() {
    int af, sock, id, cid=VMADDR_CID_HOST, len;
    struct sockaddr_vm addr;

    if ((af = VMCISock_GetAFValueFd(&id)) == -1) {
        perror("VMCISock_GetAFValueFd");
        return -1;
    }

    if ((sock = socket(af, SOCK_STREAM, 0)) == -1) {
        perror("socket");
        return -1;
    }

    memset(&addr, 0, sizeof addr);
    addr.svm_family = af;
    addr.svm_cid = cid;
    addr.svm_port = default_port;

    if ((connect(sock, (const struct sockaddr *)&addr, sizeof addr)) == -1) {
        perror("connect");
        return -1;
    }

    printf("Connected to %d:%d\n", addr.svm_cid, addr.svm_port);

    char buf[]={"ping\0"};
    if (send(sock, &buf, sizeof buf, 0) != sizeof buf) {
        perror("send");
        return -1;
    }

    if ((len = recv(sock, &buf, sizeof buf, 0)) == -1) {
        perror("recv");
        close(sock);
        return -1;
    }

    printf("recv=%s\n", buf);

    return 0;
}

int main(int argc, char **argv) {
    if (argc == 2) {
        if (strcmp(argv[1], "-c") ==  0) {
            return client();
        }
    }
    return server();
}
