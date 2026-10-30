//! Our own HTTP/1.1 server on io_uring (`SERVER=uring`). Axum's listener stays the default; both serve
//! the same `Router`, so every route behaves identically.
//!
//! - Thread per worker, each with its own ring and its own `SO_REUSEPORT` listener (the kernel
//!   load-balances connections between them).
//! - **Multishot accept**: one submission accepts connections until it is cancelled.
//! - **Multishot recv + provided buffer ring**: one submission per connection keeps receiving; the
//!   kernel picks a buffer from a ring we registered, we copy the bytes out and hand the buffer back.
//! - Requests are parsed with `httparse` and handed to the Tokio runtime (`Router::oneshot`).
//!   Response bytes (including streamed SSE chunks) come back through a queue + `eventfd`, which the
//!   ring watches with a read, and go out with `send`.

use std::{
    alloc::{Layout, alloc_zeroed},
    collections::{HashMap, VecDeque},
    io,
    net::SocketAddr,
    os::fd::{IntoRawFd, RawFd},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU16, Ordering},
        mpsc,
    },
    thread,
};

use axum::{
    Router,
    body::{Body, Bytes},
    http::{HeaderName, HeaderValue, Request, Version, header},
};
use http_body_util::BodyExt;
use io_uring::{IoUring, cqueue, opcode, squeue, types};
use socket2::{Domain, Socket, Type};
use tower::ServiceExt;

const RING_ENTRIES: u32 = 512;
const BUF_COUNT: u16 = 512;
const BUF_SIZE: usize = 4096;
const BGID: u16 = 0;
const MAX_HEAD: usize = 64 * 1024;
const MAX_BODY: usize = 1 << 20;

// user_data layout: tag (8 bits) | connection generation (32 bits) | fd (24 bits)
const ACCEPT: u64 = 1;
const RECV: u64 = 2;
const SEND: u64 = 3;
const WAKE: u64 = 4;
const CLOSE: u64 = 5;

fn ud(tag: u64, gen_: u32, fd: RawFd) -> u64 {
    (tag << 56) | ((gen_ as u64) << 24) | (fd as u64 & 0xff_ffff)
}

fn split(ud: u64) -> (u64, u32, RawFd) {
    (ud >> 56, (ud >> 24) as u32, (ud & 0xff_ffff) as RawFd)
}

/// Is io_uring usable here (kernel support, not blocked by seccomp)?
pub fn available() -> bool {
    IoUring::new(8).is_ok()
}

/// Starts `workers` threads serving `router` on `addr`. Returns the bound address.
pub fn serve(
    router: Router,
    addr: SocketAddr,
    workers: usize,
    rt: tokio::runtime::Handle,
) -> io::Result<SocketAddr> {
    let first = listener(addr)?;
    let local = first
        .local_addr()?
        .as_socket()
        .ok_or_else(|| io::Error::other("not an inet socket"))?;
    let mut sockets = vec![first];
    for _ in 1..workers.max(1) {
        sockets.push(listener(local)?);
    }
    for (i, sock) in sockets.into_iter().enumerate() {
        let (ready_tx, ready_rx) = mpsc::channel();
        let (router, rt) = (router.clone(), rt.clone());
        let lfd = sock.into_raw_fd();
        thread::Builder::new()
            .name(format!("uring-{i}"))
            .spawn(move || match Worker::new(lfd, router, rt) {
                Ok(mut w) => {
                    let _ = ready_tx.send(Ok(()));
                    w.run();
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                }
            })?;
        ready_rx
            .recv()
            .map_err(|_| io::Error::other("worker died"))??;
    }
    tracing::info!(%local, workers, "io_uring server listening");
    Ok(local)
}

fn listener(addr: SocketAddr) -> io::Result<Socket> {
    let s = Socket::new(Domain::for_address(addr), Type::STREAM, None)?;
    s.set_reuse_address(true)?;
    s.set_reuse_port(true)?;
    s.bind(&addr.into())?;
    s.listen(1024)?;
    Ok(s)
}

/// Kernel-shared ring of receive buffers (`IORING_REGISTER_PBUF_RING`).
struct BufRing {
    entries: *mut types::BufRingEntry,
    bufs: *mut u8,
    tail: u16,
}

impl BufRing {
    fn new(ring: &IoUring) -> io::Result<Self> {
        let entries_layout = Layout::from_size_align(
            BUF_COUNT as usize * std::mem::size_of::<types::BufRingEntry>(),
            4096,
        )
        .unwrap();
        let bufs_layout = Layout::from_size_align(BUF_COUNT as usize * BUF_SIZE, 4096).unwrap();
        // Lives as long as the worker thread (forever); never freed.
        let (entries, bufs) = unsafe {
            (
                alloc_zeroed(entries_layout) as *mut types::BufRingEntry,
                alloc_zeroed(bufs_layout),
            )
        };
        if entries.is_null() || bufs.is_null() {
            return Err(io::Error::other("buffer ring allocation failed"));
        }
        let mut br = Self {
            entries,
            bufs,
            tail: 0,
        };
        for bid in 0..BUF_COUNT {
            br.give_back(bid);
        }
        br.publish();
        unsafe {
            ring.submitter()
                .register_buf_ring_with_flags(entries as u64, BUF_COUNT, BGID, 0)?;
        }
        Ok(br)
    }

    fn give_back(&mut self, bid: u16) {
        let slot = (self.tail & (BUF_COUNT - 1)) as usize;
        let e = unsafe { &mut *self.entries.add(slot) };
        e.set_addr(self.bufs as u64 + bid as u64 * BUF_SIZE as u64);
        e.set_len(BUF_SIZE as u32);
        e.set_bid(bid);
        self.tail = self.tail.wrapping_add(1);
    }

    fn publish(&self) {
        unsafe {
            let tail = types::BufRingEntry::tail(self.entries) as *const AtomicU16;
            (*tail).store(self.tail, Ordering::Release);
        }
    }

    fn bytes(&self, bid: u16, len: usize) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.bufs.add(bid as usize * BUF_SIZE), len) }
    }
}

enum Out {
    Data(Bytes),
    End { keep_alive: bool },
}

/// Handler tasks (on Tokio) push response pieces here and poke the eventfd.
struct Outbox {
    queue: Mutex<Vec<(RawFd, u32, Out)>>,
    efd: RawFd,
}

impl Outbox {
    fn push(&self, fd: RawFd, gen_: u32, out: Out) {
        self.queue.lock().unwrap().push((fd, gen_, out));
        let one = 1u64;
        unsafe { libc::write(self.efd, &one as *const u64 as *const libc::c_void, 8) };
    }
}

struct Conn {
    gen_: u32,
    buf: Vec<u8>,
    pending: VecDeque<Out>,
    sent: usize,
    sending: bool,
    busy: bool,
    closing: bool,
}

struct Worker {
    ring: IoUring,
    bufs: BufRing,
    lfd: RawFd,
    outbox: Arc<Outbox>,
    wake_buf: Box<u64>,
    conns: HashMap<RawFd, Conn>,
    next_gen: u32,
    router: Router,
    rt: tokio::runtime::Handle,
}

impl Worker {
    fn new(lfd: RawFd, router: Router, rt: tokio::runtime::Handle) -> io::Result<Self> {
        let ring = IoUring::new(RING_ENTRIES)?;
        let bufs = BufRing::new(&ring)?;
        let efd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC) };
        if efd < 0 {
            return Err(io::Error::last_os_error());
        }
        let mut w = Self {
            ring,
            bufs,
            lfd,
            outbox: Arc::new(Outbox {
                queue: Mutex::default(),
                efd,
            }),
            wake_buf: Box::new(0),
            conns: HashMap::new(),
            next_gen: 1,
            router,
            rt,
        };
        w.arm_accept();
        w.arm_wake();
        w.ring.submit()?;
        Ok(w)
    }

    fn push(&mut self, e: squeue::Entry) {
        // SAFETY: every buffer referenced by an entry outlives its completion (see Conn::closing)
        while unsafe { self.ring.submission().push(&e) }.is_err() {
            self.ring.submit().expect("io_uring submit");
        }
    }

    fn arm_accept(&mut self) {
        let e = opcode::AcceptMulti::new(types::Fd(self.lfd))
            .flags(libc::SOCK_CLOEXEC)
            .build()
            .user_data(ud(ACCEPT, 0, 0));
        self.push(e);
    }

    fn arm_recv(&mut self, fd: RawFd, gen_: u32) {
        let e = opcode::RecvMulti::new(types::Fd(fd), BGID)
            .build()
            .user_data(ud(RECV, gen_, fd));
        self.push(e);
    }

    fn arm_wake(&mut self) {
        let ptr = &mut *self.wake_buf as *mut u64 as *mut u8;
        let e = opcode::Read::new(types::Fd(self.outbox.efd), ptr, 8)
            .build()
            .user_data(ud(WAKE, 0, 0));
        self.push(e);
    }

    fn run(&mut self) {
        loop {
            if let Err(e) = self.ring.submit_and_wait(1) {
                if e.raw_os_error() == Some(libc::EINTR) {
                    continue;
                }
                panic!("io_uring wait: {e}");
            }
            let cqes: Vec<cqueue::Entry> = self.ring.completion().collect();
            for cqe in cqes {
                self.complete(cqe);
            }
        }
    }

    fn complete(&mut self, cqe: cqueue::Entry) {
        let (tag, gen_, fd) = split(cqe.user_data());
        let (res, flags) = (cqe.result(), cqe.flags());
        match tag {
            ACCEPT => {
                if res >= 0 {
                    // Without this, header and body sends hit Nagle + delayed ACK: ~40 ms per response
                    let one: libc::c_int = 1;
                    unsafe {
                        libc::setsockopt(
                            res,
                            libc::IPPROTO_TCP,
                            libc::TCP_NODELAY,
                            &one as *const libc::c_int as *const libc::c_void,
                            std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                        )
                    };
                    let gen_ = self.next_gen;
                    self.next_gen = self.next_gen.wrapping_add(1).max(1);
                    self.conns.insert(
                        res,
                        Conn {
                            gen_,
                            buf: Vec::new(),
                            pending: VecDeque::new(),
                            sent: 0,
                            sending: false,
                            busy: false,
                            closing: false,
                        },
                    );
                    self.arm_recv(res, gen_);
                }
                if !cqueue::more(flags) {
                    self.arm_accept();
                }
            }
            RECV => {
                let bid = cqueue::buffer_select(flags);
                let live = self.conns.get(&fd).is_some_and(|c| c.gen_ == gen_);
                if live && res > 0 {
                    let data = self.bufs.bytes(bid.unwrap(), res as usize).to_vec();
                    self.conns
                        .get_mut(&fd)
                        .unwrap()
                        .buf
                        .extend_from_slice(&data);
                }
                if let Some(bid) = bid {
                    self.bufs.give_back(bid);
                    self.bufs.publish();
                }
                if !live {
                    return;
                }
                if res == -libc::ENOBUFS {
                    self.arm_recv(fd, gen_);
                } else if res <= 0 {
                    self.close(fd);
                } else {
                    if !cqueue::more(flags) {
                        self.arm_recv(fd, gen_);
                    }
                    self.dispatch(fd);
                }
            }
            SEND => {
                let Some(c) = self.conns.get_mut(&fd).filter(|c| c.gen_ == gen_) else {
                    return;
                };
                c.sending = false;
                if res < 0 || c.closing {
                    self.close(fd);
                    return;
                }
                c.sent += res as usize;
                if let Some(Out::Data(d)) = c.pending.front()
                    && c.sent >= d.len()
                {
                    c.pending.pop_front();
                    c.sent = 0;
                }
                self.pump(fd);
            }
            WAKE => {
                let items = std::mem::take(&mut *self.outbox.queue.lock().unwrap());
                let mut touched = Vec::new();
                for (fd, gen_, out) in items {
                    if let Some(c) = self.conns.get_mut(&fd).filter(|c| c.gen_ == gen_) {
                        c.pending.push_back(out);
                        touched.push(fd);
                    }
                }
                touched.dedup();
                for fd in touched {
                    self.pump(fd);
                }
                self.arm_wake();
            }
            _ => {} // CLOSE
        }
    }

    /// Sends the next queued piece, or finishes the response.
    fn pump(&mut self, fd: RawFd) {
        loop {
            let Some(c) = self.conns.get_mut(&fd) else {
                return;
            };
            if c.sending {
                return;
            }
            match c.pending.front() {
                None => return,
                Some(Out::Data(d)) if d.is_empty() => {
                    c.pending.pop_front();
                }
                Some(Out::Data(_))
                    if c.sent == 0 && matches!(c.pending.get(1), Some(Out::Data(_))) =>
                {
                    // Coalesce queued pieces (head + body, SSE chunk framing) into one send
                    let mut merged = Vec::new();
                    while let Some(Out::Data(d)) = c.pending.front() {
                        merged.extend_from_slice(d);
                        c.pending.pop_front();
                    }
                    c.pending.push_front(Out::Data(Bytes::from(merged)));
                }
                Some(Out::Data(d)) => {
                    let (ptr, len) = (d[c.sent..].as_ptr(), (d.len() - c.sent) as u32);
                    c.sending = true;
                    let e = opcode::Send::new(types::Fd(fd), ptr, len)
                        .flags(libc::MSG_NOSIGNAL)
                        .build()
                        .user_data(ud(SEND, c.gen_, fd));
                    self.push(e);
                    return;
                }
                Some(Out::End { keep_alive }) => {
                    let keep_alive = *keep_alive;
                    c.pending.pop_front();
                    c.busy = false;
                    if !keep_alive {
                        self.close(fd);
                        return;
                    }
                    self.dispatch(fd);
                }
            }
        }
    }

    fn close(&mut self, fd: RawFd) {
        let Some(c) = self.conns.get_mut(&fd) else {
            return;
        };
        if c.sending {
            // The kernel still reads from a buffer we own: close once the send completes
            c.closing = true;
            return;
        }
        self.conns.remove(&fd);
        unsafe { libc::shutdown(fd, libc::SHUT_RDWR) };
        let e = opcode::Close::new(types::Fd(fd))
            .build()
            .user_data(ud(CLOSE, 0, fd));
        self.push(e);
    }

    fn reject(&mut self, fd: RawFd, status: &'static str) {
        let c = self.conns.get_mut(&fd).unwrap();
        c.busy = true;
        c.buf.clear();
        let msg = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\nconnection: close\r\ncontent-length: {}\r\n\r\n{{\"error\":\"{status}\"}}",
            status.len() + 12
        );
        c.pending.push_back(Out::Data(Bytes::from(msg)));
        c.pending.push_back(Out::End { keep_alive: false });
        self.pump(fd);
    }

    /// Parses one complete request (if buffered) and hands it to the router.
    fn dispatch(&mut self, fd: RawFd) {
        let Some(c) = self.conns.get_mut(&fd) else {
            return;
        };
        if c.busy || c.closing || c.buf.is_empty() {
            return;
        }
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut req = httparse::Request::new(&mut headers);
        let head_len = match req.parse(&c.buf) {
            Ok(httparse::Status::Complete(n)) => n,
            Ok(httparse::Status::Partial) if c.buf.len() > MAX_HEAD => {
                return self.reject(fd, "431 Request Header Fields Too Large");
            }
            Ok(httparse::Status::Partial) => return,
            Err(_) => return self.reject(fd, "400 Bad Request"),
        };
        let header = |name: &str| {
            req.headers
                .iter()
                .find(|h| h.name.eq_ignore_ascii_case(name))
                .map(|h| String::from_utf8_lossy(h.value).to_ascii_lowercase())
        };
        if header("transfer-encoding").is_some() {
            return self.reject(fd, "411 Length Required");
        }
        let body_len: usize = match header("content-length").map(|v| v.trim().parse()) {
            None => 0,
            Some(Ok(n)) if n <= MAX_BODY => n,
            Some(Ok(_)) => return self.reject(fd, "413 Payload Too Large"),
            Some(Err(_)) => return self.reject(fd, "400 Bad Request"),
        };
        if c.buf.len() < head_len + body_len {
            return;
        }
        let http10 = req.version == Some(0);
        let conn_hdr = header("connection").unwrap_or_default();
        let keep_alive = if http10 {
            conn_hdr.contains("keep-alive")
        } else {
            !conn_hdr.contains("close")
        };

        let mut builder = Request::builder()
            .method(req.method.unwrap_or("GET"))
            .uri(req.path.unwrap_or("/"))
            .version(if http10 {
                Version::HTTP_10
            } else {
                Version::HTTP_11
            });
        for h in req.headers.iter() {
            if let (Ok(n), Ok(v)) = (
                HeaderName::from_bytes(h.name.as_bytes()),
                HeaderValue::from_bytes(h.value),
            ) {
                builder = builder.header(n, v);
            }
        }
        let body = Bytes::copy_from_slice(&c.buf[head_len..head_len + body_len]);
        c.buf.drain(..head_len + body_len);
        let Ok(request) = builder.body(Body::from(body)) else {
            return self.reject(fd, "400 Bad Request");
        };
        c.busy = true;

        let (gen_, outbox, router) = (c.gen_, self.outbox.clone(), self.router.clone());
        self.rt.spawn(async move {
            let res = router.oneshot(request).await.unwrap_or_else(|e| match e {});
            let (parts, mut body) = res.into_parts();
            let exact = http_body::Body::size_hint(&body).exact();
            let mut head = format!(
                "HTTP/1.1 {} {}\r\n",
                parts.status.as_u16(),
                parts.status.canonical_reason().unwrap_or("")
            );
            for (name, value) in &parts.headers {
                if name == header::CONTENT_LENGTH
                    || name == header::TRANSFER_ENCODING
                    || name == header::CONNECTION
                {
                    continue;
                }
                head.push_str(&format!(
                    "{}: {}\r\n",
                    name,
                    value.to_str().unwrap_or_default()
                ));
            }
            match exact {
                Some(n) => head.push_str(&format!("content-length: {n}\r\n")),
                None => head.push_str("transfer-encoding: chunked\r\n"),
            }
            head.push_str(if keep_alive {
                "connection: keep-alive\r\n\r\n"
            } else {
                "connection: close\r\n\r\n"
            });
            outbox.push(fd, gen_, Out::Data(Bytes::from(head)));

            let mut ok = true;
            while let Some(frame) = body.frame().await {
                let Ok(frame) = frame else {
                    ok = false;
                    break;
                };
                let Ok(data) = frame.into_data() else {
                    continue;
                };
                if data.is_empty() {
                    continue;
                }
                if exact.is_none() {
                    outbox.push(
                        fd,
                        gen_,
                        Out::Data(Bytes::from(format!("{:x}\r\n", data.len()))),
                    );
                    outbox.push(fd, gen_, Out::Data(data));
                    outbox.push(fd, gen_, Out::Data(Bytes::from_static(b"\r\n")));
                } else {
                    outbox.push(fd, gen_, Out::Data(data));
                }
            }
            if exact.is_none() && ok {
                outbox.push(fd, gen_, Out::Data(Bytes::from_static(b"0\r\n\r\n")));
            }
            outbox.push(
                fd,
                gen_,
                Out::End {
                    keep_alive: keep_alive && ok,
                },
            );
        });
    }
}
