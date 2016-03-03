.DEFAULT_GOAL := all

all: vsv

vsv: vsv.c vmci_sockets.h
	$(CC) -g -o vsv $@.c

clean:
	rm -f vsv
