extern crate libc;

use std::fmt;
use std::io;
use std::mem;
use std::fs::File;
use std::os::unix::io::{RawFd, AsRawFd};

// From linux/vm_sockets.h
const AF_VSOCK: libc::c_int = 40;
const VMADDR_CID_HOST: u32 = 2;
const SO_VM_SOCKETS_PEER_HOST_VM_ID: libc::c_int = 3;

// From vmci_sockets.h
const GET_AF_VALUE: libc::c_ulong = 0x7b8;

// From linux/vm_sockets.h
#[repr(C)]
#[derive(Copy, Clone)]
struct sockaddr_vm {
    pub svm_family: u8,
    pub svm_reserved1: u8,
    pub svm_port: u32,
    pub svm_cid: u32,
    pub svm_zero: [u8; 4],
}

// libc helpers
fn cvt(v: libc::c_int) -> io::Result<libc::c_int> {
    if v < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(v)
    }
}

fn cvt_s(v: libc::ssize_t) -> io::Result<libc::ssize_t> {
    if v < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(v)
    }
}

// Inner implements VMCISock_GetAFValueFd functionality
struct Inner {
    // The socket descriptor itself.
    s: RawFd,
    // The value to be used for the vSockets address family.
    // This value should be used as the domain argument to socket(2) (when
    // you might otherwise use AF_INET).  For vSocket-specific options,
    // this value should also be used for the level argument to
    // setsockopt(2) (when you might otherwise use SOL_TCP).
    af: libc::c_int,
    // File descriptor to the VMCI device.
    // The address family value is valid until this descriptor is closed.
    // Value is None when running on linux kernel with mainline kernel vsocket support.
    #[allow(dead_code)]
    fd: Option<File>,
}

impl Inner {
    fn new(kind: libc::c_int) -> io::Result<Inner> {
        let s: RawFd;
        let af = AF_VSOCK;
        let mut fd: Option<File> = None;

        match cvt(unsafe { libc::socket(af, kind, 0) }) {
            Ok(ret) => {
                // Modern Linux kernel with vsocket support
                s = ret;
            }
            Err(_) => {
                // Older Linux kernel without vsocket support or ESXi
                let f = try!(File::open("/dev/vsock"));
                try!(cvt(unsafe { libc::ioctl(f.as_raw_fd(), GET_AF_VALUE, &af) }));
                fd = Some(f);
                s = try!(cvt(unsafe { libc::socket(af, kind, 0) }));
            }
        };

        Ok(Inner {
            s: s,
            af: af,
            fd: fd,
        })
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        unsafe {
            libc::close(self.s);
        }
        // self.fd will also be closed at this point if set
    }
}

/// An address associated with a vsocket.
#[derive(Clone)]
pub struct SocketAddr {
    addr: sockaddr_vm,
    len: libc::socklen_t,
}

impl SocketAddr {
    fn new<F>(f: F) -> io::Result<SocketAddr>
        where F: FnOnce(*mut libc::sockaddr, *mut libc::socklen_t) -> libc::c_int
    {
        unsafe {
            let mut addr: sockaddr_vm = mem::zeroed();
            let mut len = mem::size_of::<sockaddr_vm>() as libc::socklen_t;
            try!(cvt(f(&mut addr as *mut _ as *mut _, &mut len)));

            Ok(SocketAddr {
                addr: addr,
                len: len,
            })
        }
    }
}

impl fmt::Debug for SocketAddr {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}:{}", self.addr.svm_cid, self.addr.svm_port)
    }
}

pub struct VSocketStream {
    inner: Inner,
}

impl VSocketStream {
    pub fn connect(port: u32) -> io::Result<VSocketStream> {

        let inner = try!(Inner::new(libc::SOCK_STREAM));
        let addr = sockaddr_vm {
            svm_family: inner.af as u8,
            svm_reserved1: 0,
            svm_port: port,
            svm_cid: VMADDR_CID_HOST,
            svm_zero: [0; 4],
        };
        let len = mem::size_of::<sockaddr_vm>() as u32;

        try!(cvt(unsafe { libc::connect(inner.s, &addr as *const _ as *const _, len) }));

        Ok(VSocketStream { inner: inner })
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        SocketAddr::new(|addr, len| unsafe { libc::getsockname(self.inner.s, addr, len) })
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        SocketAddr::new(|addr, len| unsafe { libc::getpeername(self.inner.s, addr, len) })
    }

    // Return the socket peer's host-specific VM ID.
    // Only available for ESX userworld endpoints.
    pub fn peer_host_vm_id(&self) -> io::Result<i32> {
        let mut id: i32 = 0;
        let mut size = mem::size_of::<i32>() as libc::socklen_t;

        try!(cvt(unsafe {
            libc::getsockopt(self.inner.s,
                             self.inner.af,
                             SO_VM_SOCKETS_PEER_HOST_VM_ID,
                             &mut id as *mut _ as *mut _,
                             &mut size as *mut _ as *mut _)
        }));

        Ok(id)
    }
}

impl io::Read for VSocketStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        io::Read::read(&mut &*self, buf)
    }
}

impl<'a> io::Read for &'a VSocketStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        unsafe {
            cvt_s(libc::recv(self.inner.s, buf.as_mut_ptr() as *mut _, buf.len(), 0))
                .map(|r| r as usize)
        }
    }
}

impl io::Write for VSocketStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        io::Write::write(&mut &*self, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        io::Write::flush(&mut &*self)
    }
}

impl<'a> io::Write for &'a VSocketStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unsafe {
            cvt_s(libc::send(self.inner.s, buf.as_ptr() as *const _, buf.len(), 0))
                .map(|r| r as usize)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub struct VSocketListener {
    inner: Inner,
}

impl VSocketListener {
    pub fn bind(port: u32) -> io::Result<VSocketListener> {
        let inner = try!(Inner::new(libc::SOCK_STREAM));
        let addr = sockaddr_vm {
            svm_family: inner.af as u8,
            svm_reserved1: 0,
            svm_port: port,
            svm_cid: VMADDR_CID_HOST,
            svm_zero: [0; 4],
        };
        let len = mem::size_of::<sockaddr_vm>() as u32;

        try!(cvt(unsafe { libc::bind(inner.s, &addr as *const _ as *const _, len) }));
        try!(cvt(unsafe { libc::listen(inner.s, 1) }));

        Ok(VSocketListener { inner: inner })
    }

    pub fn accept(&self) -> io::Result<(VSocketStream, SocketAddr)> {
        let mut fd = 0;
        let sa = try!(SocketAddr::new(|addr, len| {
            fd = unsafe { libc::accept(self.inner.s, addr, len) };
            fd
        }));

        let inner = Inner {
            s: fd,
            af: sa.addr.svm_family as libc::c_int,
            fd: None,
        };

        Ok((VSocketStream { inner: inner }, sa))
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        SocketAddr::new(|addr, len| unsafe { libc::getsockname(self.inner.s, addr, len) })
    }

    pub fn incoming<'a>(&'a self) -> Incoming<'a> {
        Incoming { listener: self }
    }
}

pub struct Incoming<'a> {
    listener: &'a VSocketListener,
}

impl<'a> Iterator for Incoming<'a> {
    type Item = io::Result<VSocketStream>;

    fn next(&mut self) -> Option<io::Result<VSocketStream>> {
        Some(self.listener.accept().map(|s| s.0))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (usize::max_value(), None)
    }
}
