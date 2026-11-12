//! Show the shell's OSD when a ZSA keyboard is plugged in or unplugged, and
//! when it changes layer.
//!
//!     jayos-moonlander-osd            watch and report changes
//!     jayos-moonlander-osd --probe    print what it sees as it changes, and exit on Ctrl-C
//!
//! The caps lock popup's counterpart for the Moonlander, run and stopped the
//! same way: hypr/moonlander-osd.sh is daemon-lifecycle.sh over this, the
//! control room's "Moonlander popups" row is a DaemonSwitchService over that,
//! and the popup is osd.sh's `show`, as jayos-lock-osd's is.
//!
//! Those are JayOS's files: the shell starts and stops this, and draws what it
//! says. This program moved here from JayOS's `rust/crates/jayos-hw` on
//! 2026-11-12 (docs/MOONLANDER.md), and the script names below are JayOS's,
//! under `config/common/scripts/` there.
//!
//! TWO SOURCES, BECAUSE ONE OF THEM IS OPTIONAL
//!
//! Plugged in and unplugged come from the kernel. A device under
//! /sys/bus/usb/devices whose idVendor is 3297 is a ZSA keyboard -- the id
//! doctor.d/50-hardware.sh looks for in the udev rules -- and its `product`
//! attribute is the name it gives itself ("Moonlander Mark I"). Both
//! attributes are readable by everyone, so this half needs no Keymapp, no
//! kontroll and no udev rule, and works on a machine with none of them.
//!
//! The layer comes from Keymapp, because nothing else on this side of the
//! cable knows it. Keymapp's API answers `GetStatus` with the keyboard it is
//! connected to and that keyboard's current layer. With Keymapp not running,
//! or its API not started -- it is off until it is switched on in Keymapp's
//! settings -- there are no layer popups, and the plug popups go on.
//!
//! NOT POLLED FOR A KEYBOARD THAT IS NOT THERE
//!
//! A plug or an unplug creates or removes a node under /dev/bus/usb/<bus>/,
//! and inotify says so; the bus is read again only then, plus once every
//! `RESCAN_EVERY` in case an event was missed. A machine with no ZSA keyboard
//! wakes this process for USB hotplug and for that backstop, twice a minute,
//! and asks Keymapp nothing. Without inotify --
//! no /dev/bus/usb, as in a container -- it reads the bus every
//! `RESCAN_BLIND`, which is the same answer later.
//!
//! THE LAYER IS POLLED, AND WHY NOT THROUGH KONTROLL
//!
//! The API has no event to wait on: `GetStatus` is its only read
//! (proto/keymapp.proto in kontroll 1.0.4), so the layer has to be asked for,
//! and asked for every `ASK_EVERY` to feel like the caps lock popup. Only
//! while a ZSA keyboard is plugged in, which is the whole of the cost.
//!
//! kontroll is ZSA's command line over that API, and moonlander.sh uses it
//! for everything it sets. Here it would be a process, a multi-threaded tokio
//! runtime and a fresh connection four times a second for a whole session, to
//! read one number. So this asks itself, over the same Unix socket kontroll
//! finds the same way: `$KEYMAPP_SOCKET`, else `.keymapp/keymapp.sock` in the
//! configuration directory -- `$XDG_CONFIG_HOME` when it is absolute,
//! otherwise `~/.config`, as kontroll's `directories` crate resolves it.
//!
//! Each question is one connection, one HTTP/2 stream and one unary call,
//! sent with prior knowledge as kontroll's tonic sends it, and the reply's
//! protobuf read field by field. One connection per question rather than one
//! held open, because a held one needs flow-control windows topped up, pings
//! answered for as long as it lives and a GOAWAY handled at any moment, and a
//! Unix-socket connect costs microseconds.
//!
//! What is deliberately NOT here is an HPACK decoder. Nothing in the reply's
//! headers is needed: a message in DATA frames is the answer, and a HEADERS
//! frame that ends the stream with no DATA before it is gRPC's trailers-only
//! reply, which is an error whatever the status inside it says. The request's
//! own headers are static-table indexes and literals that are never indexed,
//! so neither side's dynamic table is ever touched.
//!
//! A LAYER HAS TO STAY BEFORE IT IS ANNOUNCED
//!
//! A held layer key -- symbols under a thumb, released after one character --
//! is a layer change too, and announcing each of those would put a popup over
//! every bracket typed. So a new layer is announced once it has been read for
//! `SETTLE`; one that is gone again by then never was. A layer held longer
//! than that is announced, and so is the way back from it. That is the cost,
//! and the reason for the number: half a second covers a key held for one
//! character, and a switch that stays is on screen half a second after it is
//! made.
//!
//! WHAT IS NEVER ANNOUNCED
//!
//! What is already there when the daemon starts: a keyboard plugged in before
//! login, and the layer it is on. And the first layer after a gap -- a
//! keyboard plugged back in, Keymapp started, its API switched on -- which is
//! adopted silently, because a reading with nothing before it is not a change
//! anyone made. A keyboard that is plugged in says so; the layer it comes up
//! on is the one it always comes up on.
//!
//! STDOUT STAYS EMPTY, as jayos-lock-osd's does: this runs for the session, and
//! what it produces is the OSD and three files under the runtime directory:
//! `pid`, `status` for moonlander-osd.sh and doctor, and `layer` for the panel
//! a held Super shows (docs/plans/30-moonlander-layers-on-super.md). `--probe`
//! writes none of them, shows no popup and prints what it sees on stderr.
//!
//! THE LAYER FILE
//!
//! `{"layer":2,"plugged":true,"why":""}`, written whenever it changes: the
//! layer Keymapp reads the board on now -- not the settled one the popup waits
//! for, because a panel on screen shows the layer under the hands -- or null
//! with the reason there is none. It is written from readings this already
//! takes, so the panel polls nothing; it reads the file only while it is up,
//! and asks the bus itself whether a board is there, so a file left by a
//! daemon that did not stop cleanly cannot put the panel up for a board that
//! is gone. moonlander-osd.sh removes it on stop.
//!
//! A SWITCH moonlander.sh MADE IS ITS OWN TO SAY
//!
//! moonlander.sh switches to QWERTY for a game, a window or a while, and back
//! (docs/plans/31-moonlander-layout-learning.md), and says so itself, with
//! why: "QWERTY layer, for a game" says what "QWERTY layer" alone does not.
//! It notes `<layer> <epoch>` in its `announced` file as it switches, and a
//! layer this would announce within `SAID_FOR` of that note is not said a
//! second time.
//!
//! A BOARD USBGUARD HAS BLOCKED IS NOT PLUGGED IN
//!
//! Its `authorized` reads 0 until it is allowed, and it sends nothing until
//! then, so "connected" for it was a popup about a keyboard that did not type.
//! It is announced when it is allowed, as a board plugged in then -- and the
//! bus is read every `HELD_BACK_EVERY` while it waits, because allowing it
//! makes no node for inotify to see.

use jayos_moonlander::support::{self, cli, emit, paths, proc, store, sysfs};

use serde_json::Value;
use std::collections::BTreeMap;
use std::ffi::{CString, OsString};
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

/// ZSA Technology Labs' USB vendor id, as sysfs writes it.
const ZSA_VENDOR: &str = "3297";

/// What a ZSA keyboard is called when it does not say.
const UNNAMED: &str = "ZSA keyboard";

/// How often the bus is read when inotify is watching it: a backstop for an
/// event that was missed, not the way a plug is noticed.
const RESCAN_EVERY: Duration = Duration::from_secs(30);

/// How often the bus is read when nothing is watching it.
const RESCAN_BLIND: Duration = Duration::from_secs(2);

/// How long after a hotplug event the bus is read a second time. The node in
/// /dev and the attributes in /sys are made by the same device_add, but a
/// second look costs nothing and covers an attribute read before it was
/// written.
const RECHECK_AFTER: Duration = Duration::from_secs(1);

/// How often the bus is read while a ZSA keyboard on it is held back by
/// USBGuard. Allowing it writes `authorized` and makes no node under
/// /dev/bus/usb -- the node was there from the plug -- so inotify says
/// nothing, and the board waited for `RESCAN_EVERY`: up to half a minute
/// between answering the prompt and the popup. The answer comes from somebody
/// at the prompt, within seconds, and the wait is for as long as the board
/// is held back and no longer.
const HELD_BACK_EVERY: Duration = Duration::from_secs(1);

/// How often Keymapp is asked, while it answers with a keyboard.
const ASK_EVERY: Duration = Duration::from_millis(250);

/// How often, while it answers with no keyboard connected to it: Keymapp
/// connects by itself once the keyboard is plugged in, and is asked again
/// soon enough to catch that.
const ASK_IDLE: Duration = Duration::from_secs(1);

/// How often, while its socket is not there: Keymapp not running, or its API
/// not started. A connect to a socket that is not there fails at once, so
/// this is about the log a person reads in `--probe`, not about cost. A
/// socket that is there and answers badly is asked again at `ASK_IDLE`:
/// that is a Keymapp that is running, and its next answer may be a layer.
const ASK_RETRY: Duration = Duration::from_secs(3);

/// How long a new layer has to be read before it is announced. The header
/// says why there is one, and why this long.
const SETTLE: Duration = Duration::from_millis(500);

/// The whole of one question to Keymapp: connect, ask, read the answer.
/// Keymapp answers in well under a millisecond; one that takes a second is
/// stuck, and the next question is a fresh connection anyway.
const CALL_LIMIT: Duration = Duration::from_secs(1);

/// How long the layer names may take. jayos-keymapp-layers waits up to 1.5
/// seconds for Keymapp to let go of its database, so this is longer than that.
const NAMES_LIMIT: Duration = Duration::from_secs(3);

/// How long a switch moonlander.sh made and said counts as said. Its note is
/// written as it switches; the reading of that layer arrives at the next
/// question and is announced SETTLE after it, well inside this.
const SAID_FOR: Duration = Duration::from_secs(5);

/// How long each popup stays, in milliseconds, as osd.sh takes it. A layer is
/// read at a glance and changes often; a plug is rarer and has more to read.
const LAYER_MS: &str = "1200";
const PLUG_MS: &str = "2000";

/// The OSD's names for the three popups, which OsdModel.ts maps to glyphs.
/// `-off` is what makes the unplugged one red there, as it does for caps lock
/// turning off.
const ICON_PLUGGED: &str = "moonlander";
const ICON_UNPLUGGED: &str = "moonlander-off";
const ICON_LAYER: &str = "moonlander-layer";

/// What the status file says between the claim and the first look at the bus.
/// Written before the pid for the reason jayos-lock-osd's LOOKING_STATUS gives:
/// a reader that finds the pid also finds a status this process wrote.
const LOOKING_STATUS: &str = "running: looking for a ZSA keyboard";

// ── the bus ─────────────────────────────────────────────────────────────────

/// Where `/` is, for /sys and /dev. QS_USB_ROOT moves it, as it does for
/// jayos-usb; nothing but the suite sets it.
fn usb_root() -> PathBuf {
    std::env::var_os("QS_USB_ROOT")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

/// Every ZSA keyboard on the bus, by its sysfs name (`1-4.2`), with the name
/// it gives itself.
///
/// An entry with a colon is one of a device's interfaces, not a device, and
/// has no idVendor of its own. A device that disappears between the listing
/// and the read is simply not there, which is what it is.
fn zsa_keyboards(root: &Path) -> BTreeMap<String, String> {
    let dir = root.join("sys/bus/usb/devices");
    let mut out = BTreeMap::new();
    for name in sysfs::entry_names(&dir) {
        if name.contains(':') {
            continue;
        }
        let at = dir.join(&name);
        if sysfs::read(at.join("idVendor")).as_deref() != Some(ZSA_VENDOR) {
            continue;
        }
        if sysfs::read(at.join("authorized")).as_deref() == Some("0") {
            continue;
        }
        let product = sysfs::read(at.join("product"))
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| UNNAMED.to_string());
        out.insert(name, product);
    }
    out
}

/// Whether a ZSA keyboard on the bus is waiting for USBGuard to allow it.
fn zsa_held_back(root: &Path) -> bool {
    let dir = root.join("sys/bus/usb/devices");
    sysfs::entry_names(&dir).iter().filter(|name| !name.contains(':')).any(|name| {
        let at = dir.join(name);
        sysfs::read(at.join("idVendor")).as_deref() == Some(ZSA_VENDOR)
            && sysfs::read(at.join("authorized")).as_deref() == Some("0")
    })
}

/// The popups one look at the bus calls for, after the one before it.
///
/// The first look has nothing before it and calls for none: a keyboard
/// plugged in before the daemon started was not plugged in by anyone just
/// now. Unplugged before plugged, so a keyboard moved to another port within
/// one look ends on the popup that is true.
fn plug_popups(
    before: Option<&BTreeMap<String, String>>,
    now: &BTreeMap<String, String>,
) -> Vec<(&'static str, String)> {
    let Some(before) = before else { return Vec::new() };
    let mut out = Vec::new();
    for (port, name) in before {
        if !now.contains_key(port) {
            out.push((ICON_UNPLUGGED, format!("{name} disconnected")));
        }
    }
    for (port, name) in now {
        if !before.contains_key(port) {
            out.push((ICON_PLUGGED, format!("{name} connected")));
        }
    }
    out
}

/// inotify on /dev/bus/usb: the directory itself for a bus appearing, and
/// each bus directory for a device node made or removed.
struct Hotplug {
    file: File,
    dir: PathBuf,
}

impl Hotplug {
    fn new(dir: &Path) -> Option<Self> {
        // SAFETY: inotify_init1 takes flags and nothing else, and returns a
        // descriptor this process owns or -1.
        let fd = unsafe { libc::inotify_init1(libc::IN_NONBLOCK | libc::IN_CLOEXEC) };
        if fd < 0 {
            return None;
        }
        // SAFETY: fd is a fresh descriptor nothing else holds, so the File
        // is its only owner and closes it once.
        let file = unsafe { File::from_raw_fd(fd) };
        let hotplug = Hotplug { file, dir: dir.to_path_buf() };
        if !hotplug.watch(dir, libc::IN_CREATE | libc::IN_DELETE | libc::IN_ONLYDIR) {
            return None;
        }
        hotplug.watch_buses();
        Some(hotplug)
    }

    fn watch(&self, path: &Path, mask: u32) -> bool {
        let Ok(path) = CString::new(path.as_os_str().as_bytes()) else { return false };
        // SAFETY: the path is NUL-terminated and outlives the call, and the
        // descriptor is this struct's own.
        unsafe { libc::inotify_add_watch(self.file.as_raw_fd(), path.as_ptr(), mask) >= 0 }
    }

    /// A watch on every bus directory. Asking again for one already watched
    /// returns its existing watch, so this is safe to repeat after every
    /// event, which is how a bus that appears later is covered.
    fn watch_buses(&self) {
        for bus in sysfs::entries(&self.dir) {
            if bus.is_dir() {
                self.watch(&bus, libc::IN_CREATE | libc::IN_DELETE);
            }
        }
    }

    /// Wait up to `timeout` for a device node to come or go. True if one did.
    fn wait(&self, timeout: Duration) -> bool {
        let mut fds =
            [libc::pollfd { fd: self.file.as_raw_fd(), events: libc::POLLIN, revents: 0 }];
        let ms = timeout.as_millis().min(i32::MAX as u128) as libc::c_int;
        // SAFETY: one pollfd, and a count of one; poll writes only revents.
        let ready = unsafe { libc::poll(fds.as_mut_ptr(), 1, ms) };
        if ready <= 0 {
            // A timeout, or a signal during the wait. Either way the loop
            // goes round and asks again when it has to.
            return false;
        }
        // Drained, so the next wait waits. The events say which node, and
        // nothing here needs to know: the bus is read again either way.
        let mut buffer = [0u8; 4096];
        while matches!((&self.file).read(&mut buffer), Ok(n) if n > 0) {}
        self.watch_buses();
        true
    }
}

/// What the loop waits on between looks: hotplug events when inotify could
/// watch the bus, or plain time when it could not.
enum Waiter {
    Hotplug(Hotplug),
    Blind,
}

impl Waiter {
    fn new(root: &Path) -> Self {
        Hotplug::new(&root.join("dev/bus/usb")).map_or(Waiter::Blind, Waiter::Hotplug)
    }

    fn rescan_every(&self) -> Duration {
        match self {
            Waiter::Hotplug(_) => RESCAN_EVERY,
            Waiter::Blind => RESCAN_BLIND,
        }
    }

    fn wait(&self, timeout: Duration) -> bool {
        match self {
            Waiter::Hotplug(h) => h.wait(timeout),
            Waiter::Blind => {
                std::thread::sleep(timeout);
                false
            }
        }
    }
}

// ── Keymapp's API ───────────────────────────────────────────────────────────

/// Where Keymapp listens, by kontroll's rule (the header says which).
fn keymapp_socket() -> PathBuf {
    socket_from(
        std::env::var_os("KEYMAPP_SOCKET"),
        std::env::var_os("XDG_CONFIG_HOME"),
        paths::home(),
    )
}

/// `keymapp_socket` with its three inputs handed in, so a test can give it
/// each case without touching this process's environment.
///
/// A relative XDG_CONFIG_HOME is ignored, as the XDG specification says it
/// must be and as `directories` ignores it; paths::config_root does not
/// ignore one, which is why this does not call it.
fn socket_from(explicit: Option<OsString>, config: Option<OsString>, home: PathBuf) -> PathBuf {
    if let Some(socket) = explicit.filter(|s| !s.is_empty()) {
        return PathBuf::from(socket);
    }
    let config = config
        .map(PathBuf::from)
        .filter(|c| c.is_absolute())
        .unwrap_or_else(|| home.join(".config"));
    config.join(".keymapp").join("keymapp.sock")
}

/// The one call made, by its gRPC path: package `api`, service
/// `KeyboardService` (proto/keymapp.proto).
const GET_STATUS: &str = "/api.KeyboardService/GetStatus";

/// What every HTTP/2 client says first (RFC 9113 §3.4).
const PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

const DATA: u8 = 0x0;
const HEADERS: u8 = 0x1;
const RST_STREAM: u8 = 0x3;
const SETTINGS: u8 = 0x4;
const PUSH_PROMISE: u8 = 0x5;
const PING: u8 = 0x6;
const GOAWAY: u8 = 0x7;

const END_STREAM: u8 = 0x1;
const ACK: u8 = 0x1;
const END_HEADERS: u8 = 0x4;
const PADDED: u8 = 0x8;

/// The stream the call is made on: a client's first stream is 1.
const STREAM: u32 = 1;

/// The largest frame a peer may send without being told otherwise. This never
/// says otherwise, so a longer one is a peer that is not speaking HTTP/2, and
/// is refused before its length is allocated.
const MAX_FRAME: usize = 16_384;

/// One frame: a nine-byte header, then the payload (RFC 9113 §4.1).
fn frame(kind: u8, flags: u8, stream: u32, payload: &[u8]) -> Vec<u8> {
    let len = payload.len();
    let mut out = Vec::with_capacity(9 + len);
    out.extend_from_slice(&[(len >> 16) as u8, (len >> 8) as u8, len as u8, kind, flags]);
    out.extend_from_slice(&(stream & 0x7fff_ffff).to_be_bytes());
    out.extend_from_slice(payload);
    out
}

/// An HPACK integer in an `prefix`-bit prefix, `high` holding the bits above
/// it (RFC 7541 §5.1).
fn hpack_int(out: &mut Vec<u8>, high: u8, prefix: u32, value: usize) {
    let max = (1usize << prefix) - 1;
    if value < max {
        out.push(high | value as u8);
        return;
    }
    out.push(high | max as u8);
    let mut rest = value - max;
    while rest >= 0x80 {
        out.push((rest & 0x7f) as u8 | 0x80);
        rest >>= 7;
    }
    out.push(rest as u8);
}

/// An HPACK string, sent as it is rather than Huffman-coded (§5.2).
fn hpack_str(out: &mut Vec<u8>, text: &str) {
    hpack_int(out, 0x00, 7, text.len());
    out.extend_from_slice(text.as_bytes());
}

/// The request's header block.
///
/// `:method POST` and `:scheme http` are whole static-table entries, 3 and 6.
/// `:path`, `:authority` and `content-type` take their names from the static
/// table (4, 1 and 31) and their values literally, and `te` is literal
/// throughout -- all as "literal without indexing" (§6.2.2), so nothing is
/// added to the server's dynamic table and nothing here has to remember it.
/// `te: trailers` is in because gRPC requires it, and some servers refuse a
/// call without it.
fn request_headers(path: &str) -> Vec<u8> {
    let mut out = vec![0x80 | 3, 0x80 | 6];
    hpack_int(&mut out, 0x00, 4, 4);
    hpack_str(&mut out, path);
    hpack_int(&mut out, 0x00, 4, 1);
    hpack_str(&mut out, "localhost");
    hpack_int(&mut out, 0x00, 4, 31);
    hpack_str(&mut out, "application/grpc");
    out.push(0x00);
    hpack_str(&mut out, "te");
    hpack_str(&mut out, "trailers");
    out
}

/// Everything the client sends for one call, in one write: the preface, a
/// SETTINGS frame turning server push off, the HEADERS, and a DATA frame
/// holding the empty request message -- a zero compression flag and a zero
/// length -- that ends the stream.
fn request(path: &str) -> Vec<u8> {
    let mut out = PREFACE.to_vec();
    out.extend(frame(SETTINGS, 0, 0, &[0x00, 0x02, 0, 0, 0, 0]));
    out.extend(frame(HEADERS, END_HEADERS, STREAM, &request_headers(path)));
    out.extend(frame(DATA, END_STREAM, STREAM, &[0, 0, 0, 0, 0]));
    out
}

/// Fill `buffer` from the stream before `deadline`.
fn read_by(stream: &mut UnixStream, buffer: &mut [u8], deadline: Instant) -> Result<(), String> {
    let mut at = 0;
    while at < buffer.len() {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err("Keymapp did not answer in time".to_string());
        }
        stream
            .set_read_timeout(Some(left))
            .map_err(|e| format!("could not wait for Keymapp: {e}"))?;
        match stream.read(&mut buffer[at..]) {
            Ok(0) => return Err("Keymapp closed the connection before answering".to_string()),
            Ok(n) => at += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => {
                return Err("Keymapp did not answer in time".to_string());
            }
            Err(e) => return Err(format!("reading from Keymapp: {e}")),
        }
    }
    Ok(())
}

/// One frame as read: type, flags, stream and payload.
type Frame = (u8, u8, u32, Vec<u8>);

/// One frame from the server.
fn read_frame(stream: &mut UnixStream, deadline: Instant) -> Result<Frame, String> {
    let mut head = [0u8; 9];
    read_by(stream, &mut head, deadline)?;
    let len = usize::from(head[0]) << 16 | usize::from(head[1]) << 8 | usize::from(head[2]);
    if len > MAX_FRAME {
        return Err(format!("Keymapp sent a {len}-byte frame, longer than HTTP/2 allows here"));
    }
    let id = u32::from_be_bytes([head[5], head[6], head[7], head[8]]) & 0x7fff_ffff;
    let mut payload = vec![0u8; len];
    read_by(stream, &mut payload, deadline)?;
    Ok((head[3], head[4], id, payload))
}

/// A DATA frame's payload without its padding (RFC 9113 §6.1).
fn unpadded(flags: u8, payload: &[u8]) -> Result<&[u8], String> {
    if flags & PADDED == 0 {
        return Ok(payload);
    }
    let pad = usize::from(*payload.first().ok_or("Keymapp sent a padded frame with no length")?);
    if pad + 1 > payload.len() {
        return Err("Keymapp sent more padding than frame".to_string());
    }
    Ok(&payload[1..payload.len() - pad])
}

/// Send the request and collect the reply's DATA on the call's stream.
///
/// The server's SETTINGS are acknowledged and its PINGs answered, as HTTP/2
/// requires of any peer; a failure to write either is not this call's
/// failure, since a server that has gone shows up in the next read. Frames on
/// other streams, WINDOW_UPDATE, PRIORITY, CONTINUATION and any type this does
/// not know are passed over, which is what RFC 9113 §4.1 asks of an unknown
/// type and all the rest need.
fn exchange(stream: &mut UnixStream, path: &str, deadline: Instant) -> Result<Vec<u8>, String> {
    stream
        .set_write_timeout(Some(CALL_LIMIT))
        .and_then(|_| stream.write_all(&request(path)))
        .map_err(|e| format!("could not ask Keymapp: {e}"))?;
    let mut body = Vec::new();
    loop {
        let (kind, flags, id, payload) = read_frame(stream, deadline)?;
        match kind {
            SETTINGS if id == 0 && flags & ACK == 0 => {
                let _ = stream.write_all(&frame(SETTINGS, ACK, 0, &[]));
            }
            PING if flags & ACK == 0 => {
                let _ = stream.write_all(&frame(PING, ACK, 0, &payload));
            }
            GOAWAY => {
                let last = payload
                    .get(..4)
                    .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]) & 0x7fff_ffff)
                    .unwrap_or(0);
                if last < STREAM {
                    return Err("Keymapp closed the connection without taking the call".to_string());
                }
            }
            PUSH_PROMISE => return Err("Keymapp pushed a stream it was told not to".to_string()),
            RST_STREAM if id == STREAM => {
                let code = payload
                    .get(..4)
                    .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
                    .unwrap_or(0);
                return Err(format!("Keymapp cancelled the call (HTTP/2 error {code})"));
            }
            HEADERS if id == STREAM && flags & END_STREAM != 0 => {
                return if body.is_empty() {
                    Err("Keymapp answered the call with an error".to_string())
                } else {
                    Ok(body)
                };
            }
            DATA if id == STREAM => {
                body.extend_from_slice(unpadded(flags, &payload)?);
                if flags & END_STREAM != 0 {
                    return Ok(body);
                }
            }
            _ => {}
        }
    }
}

/// The message inside a gRPC reply body: a compression flag, a four-byte
/// length, then that many bytes. Nothing here offered compression, so a
/// compressed message is one this cannot read rather than one to inflate.
fn grpc_message(body: &[u8]) -> Result<&[u8], String> {
    if body.len() < 5 {
        return Err("Keymapp's answer was too short to hold a message".to_string());
    }
    if body[0] != 0 {
        return Err("Keymapp compressed its answer, which nothing asked for".to_string());
    }
    let len = u32::from_be_bytes([body[1], body[2], body[3], body[4]]) as usize;
    body.get(5..5 + len).ok_or_else(|| "Keymapp's answer was cut short".to_string())
}

/// What `GetStatus` says.
#[derive(Debug, Clone, PartialEq, Default)]
struct Status {
    keymapp: String,
    keyboard: Option<Board>,
}

/// The keyboard Keymapp is connected to.
#[derive(Debug, Clone, PartialEq, Default)]
struct Board {
    name: String,
    firmware: String,
    layer: i32,
}

/// One field's value, as the wire carries it. Fixed-width values are skipped
/// rather than kept: neither message has one.
#[derive(Debug, PartialEq)]
enum Wire<'a> {
    Varint(u64),
    Bytes(&'a [u8]),
    Fixed,
}

fn varint(buf: &[u8], at: &mut usize) -> Result<u64, String> {
    let mut value = 0u64;
    let mut shift = 0;
    loop {
        let byte = *buf.get(*at).ok_or("Keymapp's answer ends inside a number")?;
        *at += 1;
        if shift >= 64 {
            return Err("Keymapp's answer holds a number longer than ten bytes".to_string());
        }
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
        shift += 7;
    }
}

/// Every field of a protobuf message, in order, by number.
fn fields(buf: &[u8]) -> Result<Vec<(u64, Wire<'_>)>, String> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < buf.len() {
        let key = varint(buf, &mut at)?;
        let skip = |at: &mut usize, n: usize| -> Result<(), String> {
            *at = at
                .checked_add(n)
                .filter(|end| *end <= buf.len())
                .ok_or("Keymapp's answer ends inside a field")?;
            Ok(())
        };
        let value = match key & 7 {
            0 => Wire::Varint(varint(buf, &mut at)?),
            1 => {
                skip(&mut at, 8)?;
                Wire::Fixed
            }
            2 => {
                let len = usize::try_from(varint(buf, &mut at)?)
                    .map_err(|_| "a field longer than memory")?;
                let start = at;
                skip(&mut at, len)?;
                Wire::Bytes(&buf[start..at])
            }
            5 => {
                skip(&mut at, 4)?;
                Wire::Fixed
            }
            wire => {
                return Err(format!(
                    "Keymapp's answer holds wire type {wire}, which proto3 never sends"
                ))
            }
        };
        out.push((key >> 3, value));
    }
    Ok(out)
}

/// `GetStatusReply`: `keymapp_version` is field 1 and `connected_keyboard`
/// field 2, absent when Keymapp is connected to no keyboard.
fn parse_status(message: &[u8]) -> Result<Status, String> {
    let mut status = Status::default();
    for (number, value) in fields(message)? {
        match (number, value) {
            (1, Wire::Bytes(b)) => status.keymapp = String::from_utf8_lossy(b).into_owned(),
            (2, Wire::Bytes(b)) => status.keyboard = Some(parse_board(b)?),
            _ => {}
        }
    }
    Ok(status)
}

/// `ConnectedKeyboard`: `friendly_name` 1, `firmware_version` 2,
/// `current_layer` 3. An int32 below zero is sent as the ten-byte varint of
/// its 64-bit sign extension, so the low 32 bits are the value either way.
fn parse_board(message: &[u8]) -> Result<Board, String> {
    let mut board = Board::default();
    for (number, value) in fields(message)? {
        match (number, value) {
            (1, Wire::Bytes(b)) => board.name = String::from_utf8_lossy(b).into_owned(),
            (2, Wire::Bytes(b)) => board.firmware = String::from_utf8_lossy(b).into_owned(),
            (3, Wire::Varint(v)) => board.layer = v as u32 as i32,
            _ => {}
        }
    }
    Ok(board)
}

/// Ask Keymapp what it is connected to.
fn ask(socket: &Path) -> Result<Status, String> {
    let deadline = Instant::now() + CALL_LIMIT;
    let mut stream = UnixStream::connect(socket)
        .map_err(|e| format!("Keymapp's API does not answer at {}: {e}", socket.display()))?;
    let body = exchange(&mut stream, GET_STATUS, deadline)?;
    parse_status(grpc_message(&body)?)
}

// ── the layer, and when it has changed ──────────────────────────────────────

/// The layer last announced or adopted, and a different one being read.
#[derive(Debug, Default)]
struct Layers {
    current: Option<i32>,
    pending: Option<(i32, Instant)>,
}

impl Layers {
    /// One reading, or None for a gap: Keymapp not answering, connected to
    /// no keyboard, or reporting no layer. Returns the layer to announce.
    ///
    /// A gap forgets the layer, so the first reading after it is adopted
    /// rather than announced (the header's WHAT IS NEVER ANNOUNCED). A
    /// reading that differs from the current layer is announced once the same
    /// value has been read for `SETTLE`; one that changes again before then
    /// starts the wait over, and one that returns to the current layer drops
    /// it.
    fn see(&mut self, reading: Option<i32>, now: Instant) -> Option<i32> {
        let Some(layer) = reading.filter(|l| *l >= 0) else {
            *self = Layers::default();
            return None;
        };
        if self.current.is_none() || self.current == Some(layer) {
            self.current = Some(layer);
            self.pending = None;
            return None;
        }
        match self.pending {
            Some((waiting, since)) if waiting == layer => {
                if now.duration_since(since) < SETTLE {
                    return None;
                }
                self.current = Some(layer);
                self.pending = None;
                Some(layer)
            }
            _ => {
                self.pending = Some((layer, now));
                None
            }
        }
    }
}

/// Where this package's other program is: jayos-keymapp-layers is installed
/// beside this one, in /usr/bin. In jayos's paths.sh order for a package's
/// commands -- QS_PKG_BIN_DIR, then the QS_PKG_BIN paths.sh exports, then
/// /usr/bin -- so a suite or a second copy of the tree can name another.
///
/// It was QS_BIN_DIR, QS_BIN, then ~/.local/bin while both were jayos's
/// helpers, which install.sh built there; since the move on 2026-11-12 that
/// directory holds, at most, the copy an earlier install left, which jayos's
/// migrations/021 removes.
fn bin_dir() -> PathBuf {
    ["QS_PKG_BIN_DIR", "QS_PKG_BIN"]
        .iter()
        .find_map(|key| std::env::var_os(key).filter(|v| !v.is_empty()))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/usr/bin"))
}

/// The layer names Keymapp flashed, from jayos-keymapp-layers' envelope, or none.
///
/// Asked once a layer is first announced after a gap and kept until the
/// next gap: the layout changes only when Keymapp flashes one, and flashing
/// restarts the keyboard, which is a gap.
fn layer_names() -> Vec<String> {
    let helper = bin_dir().join("jayos-keymapp-layers");
    let Ok(out) = proc::output_within(&helper, &[], NAMES_LIMIT) else { return Vec::new() };
    names_from(&out.stdout)
}

fn names_from(envelope: &str) -> Vec<String> {
    serde_json::from_str::<Value>(envelope)
        .ok()
        .and_then(|v| {
            v.get("names")?
                .as_array()
                .map(|a| a.iter().map(|n| n.as_str().unwrap_or("").to_string()).collect())
        })
        .unwrap_or_default()
}

/// What the layer popup says.
///
/// "Symbols layer" for a layer with a name, and the name alone when it says
/// "layer" already, since "Nav layer layer" says it twice. An unnamed layer is
/// its number, which is the index kontroll reports and moonlanderDefaultLayer
/// takes: base is 0.
///
/// A layer titled "Layer" and nothing else is unnamed. That is the title Oryx
/// gives a layer nobody has named -- each of the three in ZSA's default
/// Moonlander layout, by Oryx's own API -- and printed alone it says "Layer"
/// for every one of them, where the number at least says which.
fn layer_message(layer: i32, names: &[String]) -> String {
    let named = usize::try_from(layer)
        .ok()
        .and_then(|i| names.get(i))
        .map(|n| n.trim())
        .filter(|n| !n.is_empty() && !n.eq_ignore_ascii_case("layer"));
    match named {
        Some(name) if name.to_lowercase().contains("layer") => name.to_string(),
        Some(name) => format!("{name} layer"),
        None => format!("Layer {layer}"),
    }
}

/// Whether moonlander.sh has said `layer` itself a moment ago: its
/// `announced` note, `<layer> <epoch>`, naming it within SAID_FOR of `now`.
fn said_already(note: &str, layer: i32, now: i64) -> bool {
    let mut words = note.split_whitespace();
    let (Some(said), Some(at)) = (words.next(), words.next()) else {
        return false;
    };
    let (Ok(said), Ok(at)) = (said.parse::<i32>(), at.parse::<i64>()) else {
        return false;
    };
    said == layer && (0..=SAID_FOR.as_secs() as i64).contains(&(now - at))
}

// ── the popup ───────────────────────────────────────────────────────────────

/// osd.sh's `show`, with the icon, the message, no value and no maximum --
/// which osd.sh fills with its own defaults -- and how long it stays.
fn osd_args(script: &Path, icon: &str, message: &str, ms: &str) -> Vec<String> {
    vec![
        script.to_string_lossy().into_owned(),
        "show".to_string(),
        icon.to_string(),
        message.to_string(),
        String::new(),
        String::new(),
        ms.to_string(),
    ]
}

/// The popups started so far, kept only so they can be reaped: jayos-lock-osd's
/// `Osd` says why a dropped Child is a zombie for the rest of the session.
#[derive(Default)]
struct Osd {
    children: Vec<Child>,
}

impl Osd {
    fn show(&mut self, icon: &str, message: &str, ms: &str) {
        self.children.retain_mut(|child| matches!(child.try_wait(), Ok(None)));
        let script = paths::script("osd.sh");
        let mut command = Command::new("bash");
        command
            .args(osd_args(&script, icon, message, ms))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        // SAFETY: setsid is async-signal-safe and touches nothing outside the
        // child, the whole of what is allowed between fork and exec. A new
        // session, for the reason jayos-lock-osd gives: a signal aimed at this
        // daemon's group should not take a popup off the screen halfway.
        unsafe {
            command.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
        match command.spawn() {
            Ok(child) => self.children.push(child),
            Err(e) => eprintln!("jayos-moonlander-osd: could not run {}: {e}", script.display()),
        }
    }
}

// ── the two files under the runtime directory ───────────────────────────────

fn run_dir() -> PathBuf {
    paths::run_dir("moonlander-osd")
}

fn write_status_in(dir: &Path, text: &str) {
    let _ = store::write_atomic(dir.join("status"), format!("{text}\n").as_bytes());
}

/// The `layer` file's line: the header's THE LAYER FILE.
fn layer_doc(plugged: bool, keymapp: &Result<Status, String>) -> String {
    let (layer, why) = if !plugged {
        (None, "no ZSA keyboard plugged in".to_string())
    } else {
        match keymapp {
            Ok(Status { keyboard: Some(board), .. }) if board.layer >= 0 => {
                (Some(board.layer), String::new())
            }
            Ok(Status { keyboard: Some(_), .. }) => (None, "Keymapp reports no layer".to_string()),
            Ok(_) => (None, "Keymapp is connected to no keyboard".to_string()),
            Err(why) => (None, why.clone()),
        }
    };
    serde_json::json!({"plugged": plugged, "layer": layer, "why": why}).to_string()
}

fn write_layer_in(dir: &Path, doc: &str) {
    let _ = store::write_atomic(dir.join("layer"), format!("{doc}\n").as_bytes());
}

fn write_pid_in(dir: &Path) {
    let _ = store::write_atomic(dir.join("pid"), format!("{}\n", std::process::id()).as_bytes());
}

/// The claim's two writes, the status before the pid, each its own writer so
/// a test can see the order and not only the result.
fn claim(dir: &Path, status: impl FnOnce(&Path), pid: impl FnOnce(&Path)) {
    status(dir);
    pid(dir);
}

/// One line saying what the daemon can see, for moonlander-osd.sh and doctor.
/// Every line begins `running:`, which is the word those read; what follows
/// says why a popup a person expected did not come.
fn status_line(boards: &BTreeMap<String, String>, keymapp: &Result<Status, String>) -> String {
    if boards.is_empty() {
        return "running: no ZSA keyboard plugged in".to_string();
    }
    let names: Vec<&str> = boards.values().map(String::as_str).collect();
    let names = names.join(", ");
    match keymapp {
        Ok(Status { keyboard: Some(_), .. }) => {
            format!("running: {names} plugged in; layers from Keymapp")
        }
        Ok(_) => format!(
            "running: {names} plugged in; Keymapp is connected to no keyboard, so no layer popups"
        ),
        Err(why) => format!("running: {names} plugged in; no layer popups: {why}"),
    }
}

// ── the loop ────────────────────────────────────────────────────────────────

fn watch(probe: bool) -> ExitCode {
    let root = usb_root();
    let socket = keymapp_socket();
    let waiter = Waiter::new(&root);
    if probe {
        match &waiter {
            Waiter::Hotplug(_) => {
                eprintln!("watching {} for hotplug", root.join("dev/bus/usb").display())
            }
            Waiter::Blind => eprintln!(
                "cannot watch {}; reading the bus every {}s",
                root.join("dev/bus/usb").display(),
                RESCAN_BLIND.as_secs()
            ),
        }
        eprintln!("asking Keymapp at {}", socket.display());
    }

    let mut osd = Osd::default();
    let mut boards: Option<BTreeMap<String, String>> = None;
    let mut layers = Layers::default();
    let mut names: Option<Vec<String>> = None;
    let mut keymapp: Result<Status, String> = Err("not asked yet".to_string());
    let mut said = String::new();
    let mut layer_said = String::new();
    let mut heard = String::new();
    let mut next_scan = Instant::now();
    let mut next_ask = Instant::now();
    let mut recheck = false;

    loop {
        let now = Instant::now();
        if now >= next_scan {
            let seen = zsa_keyboards(&root);
            for (icon, message) in plug_popups(boards.as_ref(), &seen) {
                if probe {
                    eprintln!("  {message}");
                } else {
                    osd.show(icon, &message, PLUG_MS);
                }
            }
            if seen.is_empty() {
                layers.see(None, now);
                names = None;
                keymapp = Err("no ZSA keyboard plugged in".to_string());
            } else if boards.as_ref().is_none_or(|b| b.is_empty()) {
                next_ask = now;
            }
            boards = Some(seen);
            next_scan = now
                + if recheck {
                    RECHECK_AFTER
                } else if zsa_held_back(&root) {
                    HELD_BACK_EVERY
                } else {
                    waiter.rescan_every()
                };
            recheck = false;
        }

        let plugged = boards.as_ref().is_some_and(|b| !b.is_empty());
        if plugged && now >= next_ask {
            keymapp = ask(&socket);
            let (reading, again) = match &keymapp {
                Ok(Status { keyboard: Some(board), .. }) => (Some(board.layer), ASK_EVERY),
                Ok(_) => (None, ASK_IDLE),
                Err(_) if socket.exists() => (None, ASK_IDLE),
                Err(_) => (None, ASK_RETRY),
            };
            if reading.is_none() {
                names = None;
            }
            if probe {
                let line = match &keymapp {
                    Ok(Status { keyboard: Some(b), keymapp: v }) => {
                        format!("Keymapp {v}: {} on layer {}", b.name, b.layer)
                    }
                    Ok(Status { keymapp: v, .. }) => {
                        format!("Keymapp {v}: connected to no keyboard")
                    }
                    Err(why) => why.clone(),
                };
                if line != heard {
                    eprintln!("  {line}");
                    heard = line;
                }
            }
            if let Some(layer) = layers.see(reading, now) {
                let note = std::fs::read_to_string(paths::run_dir("moonlander").join("announced"))
                    .unwrap_or_default();
                let names = names.get_or_insert_with(layer_names);
                let message = layer_message(layer, names);
                if said_already(&note, layer, support::now_unix()) {
                    if probe {
                        eprintln!("  {message}, said by moonlander.sh");
                    }
                } else if probe {
                    eprintln!("  {message}");
                } else {
                    osd.show(ICON_LAYER, &message, LAYER_MS);
                }
            }
            next_ask = now + again;
        }

        if !probe {
            let line = status_line(boards.as_ref().unwrap_or(&BTreeMap::new()), &keymapp);
            if line != said {
                write_status_in(&run_dir(), &line);
                said = line;
            }
            let doc = layer_doc(plugged, &keymapp);
            if doc != layer_said {
                write_layer_in(&run_dir(), &doc);
                layer_said = doc;
            }
        }

        let until = if plugged { next_scan.min(next_ask) } else { next_scan };
        if waiter.wait(until.saturating_duration_since(Instant::now())) {
            next_scan = Instant::now();
            recheck = true;
        }
    }
}

fn main() -> ExitCode {
    // `--probe` from anywhere in argv and nothing else looked at, for the
    // reason jayos-lock-osd gives at its own `has("probe")`: a daemon that
    // refused to start over a mistyped flag in a login script would leave the
    // popups off for the session. jayos-core's cli.rs table in jayos named both.
    let probe = cli::Args::from_env().has("probe");
    if !probe {
        let dir = run_dir();
        claim(&dir, |d| write_status_in(d, LOOKING_STATUS), write_pid_in);
        if !dir.join("pid").exists() {
            return emit::failure(format!("could not write {}", dir.join("pid").display()));
        }
    }
    watch(probe)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::thread;

    fn tmpdir(test: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("qs-moonlander-osd-{}-{test}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    // ── the bus ─────────────────────────────────────────────────────────────

    fn device(root: &Path, name: &str, vendor: &str, product: Option<&str>) {
        let at = root.join("sys/bus/usb/devices").join(name);
        fs::create_dir_all(&at).unwrap();
        fs::write(at.join("idVendor"), format!("{vendor}\n")).unwrap();
        if let Some(p) = product {
            fs::write(at.join("product"), format!("{p}\n")).unwrap();
        }
    }

    #[test]
    fn a_layer_moonlander_sh_has_just_said_is_not_said_again() {
        assert!(said_already("0 1000\n", 0, 1000));
        assert!(said_already("0 1000", 0, 1005), "within SAID_FOR");
        assert!(!said_already("0 1000", 0, 1006), "and not after it");
        assert!(!said_already("0 1000", 1, 1001), "another layer is the board's own");
        assert!(!said_already("0 1000", 0, 999), "nor a note from the future");
        assert!(!said_already("", 0, 1000));
        assert!(!said_already("zero 1000", 0, 1000));
        assert!(!said_already("0", 0, 1000));
    }

    #[test]
    fn only_zsa_devices_are_keyboards_here_and_interfaces_are_not_devices() {
        let root = tmpdir("bus");
        device(&root, "1-4", "3297", Some("Moonlander Mark I"));
        device(&root, "1-4:1.0", "3297", Some("an interface"));
        device(&root, "1-5", "046d", Some("USB Receiver"));
        device(&root, "2-1", "3297", None);
        device(&root, "usb1", "1d6b", Some("xHCI Host Controller"));
        let found = zsa_keyboards(&root);
        assert_eq!(
            found.into_iter().collect::<Vec<_>>(),
            vec![
                ("1-4".to_string(), "Moonlander Mark I".to_string()),
                ("2-1".to_string(), UNNAMED.to_string()),
            ]
        );
    }

    #[test]
    fn a_board_usbguard_has_not_allowed_is_not_plugged_in() {
        let root = tmpdir("bus-authorized");
        device(&root, "1-4", "3297", Some("Moonlander Mark I"));
        device(&root, "1-6", "3297", Some("Voyager"));
        fs::write(root.join("sys/bus/usb/devices/1-4/authorized"), "0\n").unwrap();
        fs::write(root.join("sys/bus/usb/devices/1-6/authorized"), "1\n").unwrap();
        let found = zsa_keyboards(&root);
        assert_eq!(
            found.into_iter().collect::<Vec<_>>(),
            vec![("1-6".to_string(), "Voyager".to_string())]
        );
        // Allowed, it is a board plugged in then, and is announced so.
        let before = zsa_keyboards(&root);
        fs::write(root.join("sys/bus/usb/devices/1-4/authorized"), "1\n").unwrap();
        let after = zsa_keyboards(&root);
        assert_eq!(
            plug_popups(Some(&before), &after),
            vec![(ICON_PLUGGED, "Moonlander Mark I connected".to_string())]
        );
    }

    #[test]
    fn a_board_held_back_is_looked_at_until_it_is_allowed() {
        let root = tmpdir("bus-held-back");
        device(&root, "1-4", "3297", Some("Moonlander Mark I"));
        device(&root, "1-5", "046d", Some("USB Receiver"));
        fs::write(root.join("sys/bus/usb/devices/1-5/authorized"), "0\n").unwrap();
        assert!(!zsa_held_back(&root), "another vendor's blocked device is not a board waiting");
        fs::write(root.join("sys/bus/usb/devices/1-4/authorized"), "0\n").unwrap();
        assert!(zsa_held_back(&root), "a ZSA board USBGuard blocked is");
        fs::write(root.join("sys/bus/usb/devices/1-4/authorized"), "1\n").unwrap();
        assert!(!zsa_held_back(&root), "and once allowed it is not");
        fs::create_dir_all(root.join("sys/bus/usb/devices/1-4:1.0")).unwrap();
        fs::write(root.join("sys/bus/usb/devices/1-4:1.0/authorized"), "0\n").unwrap();
        assert!(!zsa_held_back(&root), "an interface's own flag is not the board's");
    }

    fn layer_of(doc: &str) -> (Value, Value, Value) {
        let v: Value = serde_json::from_str(doc).expect("the layer file is JSON");
        (v["plugged"].clone(), v["layer"].clone(), v["why"].clone())
    }

    #[test]
    fn the_layer_file_says_the_layer_keymapp_reads_now() {
        let board = Board { name: "Moonlander MK1".into(), firmware: "x".into(), layer: 2 };
        let status = Ok(Status { keymapp: "2.0".into(), keyboard: Some(board) });
        assert_eq!(
            layer_of(&layer_doc(true, &status)),
            (Value::Bool(true), serde_json::json!(2), serde_json::json!(""))
        );
        // One line, since the panel reads it whole.
        assert!(!layer_doc(true, &status).contains('\n'));
    }

    #[test]
    fn with_no_layer_the_layer_file_says_why() {
        let none = Ok(Status { keymapp: "2.0".into(), keyboard: None });
        let (plugged, layer, why) = layer_of(&layer_doc(true, &none));
        assert_eq!((plugged, layer), (Value::Bool(true), Value::Null));
        assert_eq!(why, "Keymapp is connected to no keyboard");

        let refused: Result<Status, String> = Err("Keymapp's API does not answer at /x".into());
        let (_, layer, why) = layer_of(&layer_doc(true, &refused));
        assert_eq!(
            (layer, why),
            (Value::Null, serde_json::json!("Keymapp's API does not answer at /x"))
        );

        let board = Board { name: "M".into(), firmware: "x".into(), layer: -1 };
        let negative = Ok(Status { keymapp: "2.0".into(), keyboard: Some(board) });
        let (_, layer, why) = layer_of(&layer_doc(true, &negative));
        assert_eq!((layer, why), (Value::Null, serde_json::json!("Keymapp reports no layer")));
    }

    #[test]
    fn unplugged_the_layer_file_has_no_layer_whatever_keymapp_last_said() {
        let board = Board { name: "M".into(), firmware: "x".into(), layer: 3 };
        let stale = Ok(Status { keymapp: "2.0".into(), keyboard: Some(board) });
        assert_eq!(
            layer_of(&layer_doc(false, &stale)),
            (Value::Bool(false), Value::Null, serde_json::json!("no ZSA keyboard plugged in"))
        );
    }

    #[test]
    fn no_bus_is_no_keyboard_rather_than_an_error() {
        assert!(zsa_keyboards(&tmpdir("nobus")).is_empty());
    }

    fn map(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
        entries.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn the_first_look_announces_nothing() {
        assert!(plug_popups(None, &map(&[("1-4", "Moonlander Mark I")])).is_empty());
    }

    #[test]
    fn a_keyboard_plugged_in_and_unplugged_says_so() {
        let none = map(&[]);
        let one = map(&[("1-4", "Moonlander Mark I")]);
        assert_eq!(
            plug_popups(Some(&none), &one),
            vec![(ICON_PLUGGED, "Moonlander Mark I connected".to_string())]
        );
        assert_eq!(
            plug_popups(Some(&one), &none),
            vec![(ICON_UNPLUGGED, "Moonlander Mark I disconnected".to_string())]
        );
        assert!(plug_popups(Some(&one), &one).is_empty());
    }

    #[test]
    fn a_keyboard_moved_to_another_port_ends_on_connected() {
        let before = map(&[("1-4", "Moonlander Mark I")]);
        let after = map(&[("3-2", "Moonlander Mark I")]);
        let popups = plug_popups(Some(&before), &after);
        assert_eq!(popups.len(), 2);
        assert_eq!(popups.last().unwrap().0, ICON_PLUGGED);
    }

    #[test]
    fn a_device_node_made_in_a_watched_bus_wakes_the_wait() {
        let root = tmpdir("inotify");
        let dir = root.join("dev/bus/usb");
        fs::create_dir_all(dir.join("001")).unwrap();
        let hotplug = Hotplug::new(&dir).expect("inotify on a temporary directory");
        assert!(!hotplug.wait(Duration::from_millis(50)), "nothing happened yet");
        fs::write(dir.join("001/005"), b"").unwrap();
        assert!(hotplug.wait(Duration::from_secs(2)), "a node made in a bus directory");
        assert!(!hotplug.wait(Duration::from_millis(50)), "and the wait was drained");
        fs::create_dir_all(dir.join("002")).unwrap();
        assert!(hotplug.wait(Duration::from_secs(2)), "a new bus");
        fs::write(dir.join("002/003"), b"").unwrap();
        assert!(hotplug.wait(Duration::from_secs(2)), "and a node on the bus that appeared later");
        fs::remove_file(dir.join("001/005")).unwrap();
        assert!(hotplug.wait(Duration::from_secs(2)), "a node removed");
    }

    #[test]
    fn no_bus_directory_is_a_blind_waiter() {
        assert!(matches!(Waiter::new(&tmpdir("blind")), Waiter::Blind));
    }

    // ── where Keymapp is ────────────────────────────────────────────────────

    #[test]
    fn the_socket_is_found_the_way_kontroll_finds_it() {
        let home = PathBuf::from("/home/me");
        assert_eq!(
            socket_from(Some("/run/k.sock".into()), Some("/cfg".into()), home.clone()),
            PathBuf::from("/run/k.sock")
        );
        assert_eq!(
            socket_from(None, Some("/cfg".into()), home.clone()),
            PathBuf::from("/cfg/.keymapp/keymapp.sock")
        );
        assert_eq!(
            socket_from(Some("".into()), None, home.clone()),
            PathBuf::from("/home/me/.config/.keymapp/keymapp.sock")
        );
        assert_eq!(
            socket_from(None, Some("relative/cfg".into()), home),
            PathBuf::from("/home/me/.config/.keymapp/keymapp.sock")
        );
    }

    // ── what is sent ────────────────────────────────────────────────────────

    #[test]
    fn hpack_integers_are_the_rfcs_examples() {
        let mut out = Vec::new();
        hpack_int(&mut out, 0, 5, 10);
        assert_eq!(out, [0x0a]);
        out.clear();
        hpack_int(&mut out, 0, 5, 1337);
        assert_eq!(out, [0x1f, 0x9a, 0x0a]);
        out.clear();
        hpack_int(&mut out, 0, 8, 42);
        assert_eq!(out, [0x2a]);
        out.clear();
        hpack_int(&mut out, 0, 4, 31);
        assert_eq!(out, [0x0f, 0x10]);
    }

    #[test]
    fn the_request_headers_are_these_bytes() {
        let mut want = vec![0x83, 0x86, 0x04, 30];
        want.extend_from_slice(b"/api.KeyboardService/GetStatus");
        want.extend_from_slice(&[0x01, 9]);
        want.extend_from_slice(b"localhost");
        want.extend_from_slice(&[0x0f, 0x10, 16]);
        want.extend_from_slice(b"application/grpc");
        want.extend_from_slice(&[0x00, 2]);
        want.extend_from_slice(b"te");
        want.push(8);
        want.extend_from_slice(b"trailers");
        assert_eq!(request_headers(GET_STATUS), want);
    }

    #[test]
    fn the_request_is_preface_settings_headers_and_an_empty_message() {
        let bytes = request(GET_STATUS);
        assert!(bytes.starts_with(PREFACE));
        let rest = &bytes[PREFACE.len()..];
        assert_eq!(&rest[..9], &[0, 0, 6, SETTINGS, 0, 0, 0, 0, 0]);
        assert_eq!(&rest[9..15], &[0, 2, 0, 0, 0, 0]);
        let headers = &rest[15..];
        let len = usize::from(headers[2]);
        assert_eq!(&headers[3..9], &[HEADERS, END_HEADERS, 0, 0, 0, 1]);
        let data = &headers[9 + len..];
        assert_eq!(data, &[0, 0, 5, DATA, END_STREAM, 0, 0, 0, 1, 0, 0, 0, 0, 0]);
    }

    // ── what comes back ─────────────────────────────────────────────────────

    fn pb_varint(mut v: u64, out: &mut Vec<u8>) {
        while v >= 0x80 {
            out.push((v & 0x7f) as u8 | 0x80);
            v >>= 7;
        }
        out.push(v as u8);
    }

    fn pb_bytes(field: u64, bytes: &[u8], out: &mut Vec<u8>) {
        pb_varint(field << 3 | 2, out);
        pb_varint(bytes.len() as u64, out);
        out.extend_from_slice(bytes);
    }

    fn reply(version: &str, board: Option<(&str, &str, i32)>) -> Vec<u8> {
        let mut out = Vec::new();
        pb_bytes(1, version.as_bytes(), &mut out);
        if let Some((name, firmware, layer)) = board {
            let mut b = Vec::new();
            pb_bytes(1, name.as_bytes(), &mut b);
            pb_bytes(2, firmware.as_bytes(), &mut b);
            if layer != 0 {
                pb_varint(3 << 3, &mut b);
                pb_varint(layer as i64 as u64, &mut b);
            }
            pb_bytes(2, &b, &mut out);
        }
        out
    }

    #[test]
    fn a_status_with_a_keyboard_reads_its_layer() {
        let status = parse_status(&reply("1.3.4", Some(("Moonlander MK1", "abc/def", 2)))).unwrap();
        assert_eq!(status.keymapp, "1.3.4");
        assert_eq!(
            status.keyboard,
            Some(Board { name: "Moonlander MK1".into(), firmware: "abc/def".into(), layer: 2 })
        );
    }

    #[test]
    fn layer_zero_is_not_sent_and_reads_as_zero() {
        let status = parse_status(&reply("1.3.4", Some(("Moonlander MK1", "x", 0)))).unwrap();
        assert_eq!(status.keyboard.unwrap().layer, 0);
    }

    #[test]
    fn no_keyboard_field_is_no_keyboard() {
        assert_eq!(parse_status(&reply("1.3.4", None)).unwrap().keyboard, None);
        assert_eq!(parse_status(&[]).unwrap(), Status::default());
    }

    #[test]
    fn a_negative_layer_is_its_ten_byte_varint() {
        let status = parse_status(&reply("1", Some(("k", "f", -1)))).unwrap();
        assert_eq!(status.keyboard.unwrap().layer, -1);
    }

    #[test]
    fn fields_this_does_not_know_are_skipped_whatever_their_wire_type() {
        let mut message = Vec::new();
        pb_varint(9 << 3, &mut message);
        pb_varint(300, &mut message);
        pb_varint(10 << 3 | 1, &mut message);
        message.extend_from_slice(&[1; 8]);
        pb_varint(11 << 3 | 5, &mut message);
        message.extend_from_slice(&[2; 4]);
        pb_bytes(12, b"later", &mut message);
        message.extend(reply("1.3.4", Some(("k", "f", 3))));
        assert_eq!(parse_status(&message).unwrap().keyboard.unwrap().layer, 3);
    }

    #[test]
    fn a_cut_or_malformed_message_is_an_error_not_a_layer() {
        let whole = reply("1.3.4", Some(("Moonlander MK1", "f", 2)));
        assert!(parse_status(&whole[..whole.len() - 1]).is_err());
        assert!(parse_status(&[0x80]).is_err());
        assert!(parse_status(&[1 << 3 | 3]).is_err(), "a group, which proto3 never sends");
        assert!(parse_status(&[
            0x08, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01
        ])
        .is_err());
    }

    #[test]
    fn a_grpc_body_is_flag_length_message() {
        let mut body = vec![0, 0, 0, 0, 3, 7, 8, 9];
        assert_eq!(grpc_message(&body).unwrap(), &[7, 8, 9]);
        body[0] = 1;
        assert!(grpc_message(&body).is_err(), "compressed");
        assert!(grpc_message(&[0, 0, 0, 0, 4, 1]).is_err(), "cut short");
        assert!(grpc_message(&[0, 0]).is_err(), "too short for a length");
    }

    #[test]
    fn padding_is_taken_off_and_too_much_of_it_is_an_error() {
        assert_eq!(unpadded(PADDED, &[2, 7, 8, 0, 0]).unwrap(), &[7, 8]);
        assert_eq!(unpadded(0, &[2, 7, 8]).unwrap(), &[2, 7, 8]);
        assert!(unpadded(PADDED, &[5, 1]).is_err());
        assert!(unpadded(PADDED, &[]).is_err());
    }

    // ── a whole call, against a server played by the test ───────────────────

    /// What a test server reads from the client: the preface, then frames,
    /// until the request's DATA has ended the stream.
    fn read_request(server: &mut UnixStream) -> Vec<Frame> {
        let mut preface = [0u8; 24];
        server.read_exact(&mut preface).unwrap();
        assert_eq!(&preface[..], PREFACE);
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut frames = Vec::new();
        loop {
            let f = read_frame(server, deadline).unwrap();
            let done = f.0 == DATA && f.1 & END_STREAM != 0;
            frames.push(f);
            if done {
                return frames;
            }
        }
    }

    /// Run the client against a server thread that answers with `script`,
    /// and hand back the client's answer and every frame the server read.
    fn call(
        script: impl FnOnce(&mut UnixStream) + Send + 'static,
    ) -> (Result<Vec<u8>, String>, Vec<Frame>) {
        let (mut client, mut server) = UnixStream::pair().unwrap();
        let handle = thread::spawn(move || {
            let mut frames = read_request(&mut server);
            script(&mut server);
            server.set_read_timeout(Some(Duration::from_millis(200))).unwrap();
            let deadline = Instant::now() + Duration::from_millis(200);
            while let Ok(f) = read_frame(&mut server, deadline) {
                frames.push(f);
            }
            frames
        });
        let answer = exchange(&mut client, GET_STATUS, Instant::now() + Duration::from_secs(2));
        drop(client);
        (answer, handle.join().unwrap())
    }

    fn message_body(message: &[u8]) -> Vec<u8> {
        let mut body = vec![0];
        body.extend_from_slice(&(message.len() as u32).to_be_bytes());
        body.extend_from_slice(message);
        body
    }

    fn send(server: &mut UnixStream, kind: u8, flags: u8, stream: u32, payload: &[u8]) {
        server.write_all(&frame(kind, flags, stream, payload)).unwrap();
    }

    #[test]
    fn a_whole_call_returns_the_message_and_acknowledges_the_servers_settings() {
        let message = reply("1.3.4", Some(("Moonlander MK1", "f", 4)));
        let body = message_body(&message);
        let (answer, frames) = call(move |s| {
            send(s, SETTINGS, 0, 0, &[0, 3, 0, 0, 0, 100]);
            send(s, SETTINGS, ACK, 0, &[]);
            send(s, HEADERS, END_HEADERS, STREAM, &[0x88]);
            send(s, DATA, 0, STREAM, &body);
            send(s, HEADERS, END_HEADERS | END_STREAM, STREAM, &[0x40, 1, b'x', 1, b'0']);
        });
        let body = answer.unwrap();
        assert_eq!(parse_status(grpc_message(&body).unwrap()).unwrap().keyboard.unwrap().layer, 4);
        assert!(frames.iter().any(|f| f.0 == SETTINGS && f.1 == ACK && f.3.is_empty()));
        assert!(frames.iter().any(|f| f.0 == HEADERS && f.2 == STREAM));
    }

    #[test]
    fn a_ping_is_answered_with_its_own_bytes() {
        let body = message_body(&reply("1", None));
        let (answer, frames) = call(move |s| {
            send(s, PING, 0, 0, b"12345678");
            send(s, HEADERS, END_HEADERS, STREAM, &[0x88]);
            send(s, DATA, 0, STREAM, &body);
            send(s, HEADERS, END_HEADERS | END_STREAM, STREAM, &[0x88]);
        });
        assert!(answer.is_ok());
        assert!(frames.iter().any(|f| f.0 == PING && f.1 == ACK && f.3 == b"12345678"));
    }

    #[test]
    fn data_split_and_padded_and_other_streams_and_types_are_handled() {
        let body = message_body(&reply("1.3.4", Some(("k", "f", 5))));
        let (first, second) = body.split_at(3);
        let (first, second) = (first.to_vec(), second.to_vec());
        let (answer, _) = call(move |s| {
            send(s, HEADERS, 0, STREAM, &[0x88]);
            send(s, 0x9, END_HEADERS, STREAM, &[0x40, 1, b'a', 1, b'b']);
            send(s, 0x8, 0, 0, &[0, 0, 1, 0]);
            send(s, DATA, 0, 3, b"not ours");
            send(s, 0xfa, 0, 0, b"a type nobody knows");
            let mut padded = vec![2];
            padded.extend_from_slice(&first);
            padded.extend_from_slice(&[0, 0]);
            send(s, DATA, PADDED, STREAM, &padded);
            send(s, DATA, 0, STREAM, &second);
            send(s, HEADERS, END_HEADERS | END_STREAM, STREAM, &[0x88]);
        });
        let body = answer.unwrap();
        assert_eq!(parse_status(grpc_message(&body).unwrap()).unwrap().keyboard.unwrap().layer, 5);
    }

    #[test]
    fn a_trailers_only_reply_is_an_error() {
        let (answer, _) = call(|s| {
            send(s, HEADERS, END_HEADERS | END_STREAM, STREAM, &[0x88]);
        });
        assert!(answer.unwrap_err().contains("error"));
    }

    #[test]
    fn a_reset_a_goaway_before_the_call_and_a_push_are_errors() {
        let (answer, _) = call(|s| send(s, RST_STREAM, 0, STREAM, &[0, 0, 0, 8]));
        assert!(answer.unwrap_err().contains("error 8"));
        let (answer, _) = call(|s| send(s, GOAWAY, 0, 0, &[0, 0, 0, 0, 0, 0, 0, 0]));
        assert!(answer.is_err());
        let (answer, _) = call(|s| send(s, PUSH_PROMISE, END_HEADERS, STREAM, &[0, 0, 0, 2, 0x82]));
        assert!(answer.is_err());
    }

    #[test]
    fn a_goaway_after_the_call_still_lets_it_finish() {
        let body = message_body(&reply("1", Some(("k", "f", 1))));
        let (answer, _) = call(move |s| {
            send(s, GOAWAY, 0, 0, &[0, 0, 0, 1, 0, 0, 0, 0]);
            send(s, DATA, 0, STREAM, &body);
            send(s, HEADERS, END_HEADERS | END_STREAM, STREAM, &[0x88]);
        });
        assert!(answer.is_ok());
    }

    #[test]
    fn a_server_that_hangs_up_or_says_nothing_is_an_error_in_time() {
        let (answer, _) = call(|_| {});
        assert!(answer.unwrap_err().contains("closed"));

        let (mut client, mut server) = UnixStream::pair().unwrap();
        let started = Instant::now();
        let handle = thread::spawn(move || {
            let _ = read_request(&mut server);
            thread::sleep(Duration::from_millis(600));
        });
        let answer = exchange(&mut client, GET_STATUS, Instant::now() + Duration::from_millis(200));
        assert!(answer.unwrap_err().contains("in time"));
        assert!(started.elapsed() < Duration::from_millis(500));
        handle.join().unwrap();
    }

    #[test]
    fn a_frame_longer_than_allowed_is_refused_before_it_is_read() {
        let (answer, _) = call(|s| {
            s.write_all(&[0x01, 0x00, 0x00, DATA, 0, 0, 0, 0, 1]).unwrap();
        });
        assert!(answer.unwrap_err().contains("frame"));
    }

    #[test]
    fn ask_over_a_socket_that_is_not_there_says_where_it_looked() {
        let socket = tmpdir("nosocket").join("keymapp.sock");
        let why = ask(&socket).unwrap_err();
        assert!(why.contains(&socket.display().to_string()), "{why}");
    }

    // ── when a layer is announced ───────────────────────────────────────────

    #[test]
    fn the_first_reading_and_the_first_after_a_gap_are_adopted() {
        let t = Instant::now();
        let mut layers = Layers::default();
        assert_eq!(layers.see(Some(2), t), None);
        assert_eq!(layers.current, Some(2));
        assert_eq!(layers.see(None, t), None);
        assert_eq!(layers.see(Some(1), t + SETTLE * 4), None);
        assert_eq!(layers.current, Some(1));
    }

    #[test]
    fn a_layer_is_announced_once_it_has_stayed() {
        let t = Instant::now();
        let mut layers = Layers::default();
        layers.see(Some(0), t);
        assert_eq!(layers.see(Some(2), t + ASK_EVERY), None);
        assert_eq!(layers.see(Some(2), t + ASK_EVERY * 2), None);
        assert_eq!(layers.see(Some(2), t + ASK_EVERY + SETTLE), Some(2));
        assert_eq!(layers.see(Some(2), t + ASK_EVERY * 8), None, "and only once");
    }

    #[test]
    fn a_layer_gone_before_it_settled_is_never_announced() {
        let t = Instant::now();
        let mut layers = Layers::default();
        layers.see(Some(0), t);
        layers.see(Some(1), t + ASK_EVERY);
        assert_eq!(layers.see(Some(0), t + ASK_EVERY * 2), None);
        assert_eq!(layers.see(Some(0), t + ASK_EVERY * 9), None);
        assert_eq!(layers.current, Some(0));
    }

    #[test]
    fn a_layer_that_changes_again_starts_the_wait_over() {
        let t = Instant::now();
        let mut layers = Layers::default();
        layers.see(Some(0), t);
        layers.see(Some(1), t + ASK_EVERY);
        assert_eq!(layers.see(Some(2), t + ASK_EVERY + SETTLE), None);
        assert_eq!(layers.see(Some(2), t + ASK_EVERY * 2 + SETTLE), None);
        assert_eq!(layers.see(Some(2), t + ASK_EVERY + SETTLE * 2), Some(2));
    }

    #[test]
    fn a_negative_layer_is_a_gap() {
        let t = Instant::now();
        let mut layers = Layers::default();
        layers.see(Some(3), t);
        assert_eq!(layers.see(Some(-1), t + SETTLE * 2), None);
        assert_eq!(layers.current, None);
    }

    // ── what the popups say ─────────────────────────────────────────────────

    #[test]
    fn a_layer_is_named_when_keymapp_names_it() {
        let names: Vec<String> = ["Base", "Symbols", "Layer 2", "  ", "Nav layer"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(layer_message(1, &names), "Symbols layer");
        assert_eq!(layer_message(2, &names), "Layer 2");
        assert_eq!(layer_message(3, &names), "Layer 3");
        assert_eq!(layer_message(4, &names), "Nav layer");
        assert_eq!(layer_message(7, &names), "Layer 7");
        assert_eq!(layer_message(0, &[]), "Layer 0");
    }

    #[test]
    fn a_layer_titled_only_layer_is_its_number() {
        // Oryx's title for a layer nobody named, on every layer of ZSA's
        // default layout. Alone, every popup would say "Layer".
        let names: Vec<String> =
            ["Layer", " layer ", "LAYER"].iter().map(|s| s.to_string()).collect();
        assert_eq!(layer_message(0, &names), "Layer 0");
        assert_eq!(layer_message(1, &names), "Layer 1");
        assert_eq!(layer_message(2, &names), "Layer 2");
    }

    #[test]
    fn the_names_are_read_from_the_helpers_envelope() {
        assert_eq!(
            names_from(r#"{"names":["Base","Symbols"],"error":""}"#),
            vec!["Base", "Symbols"]
        );
        assert!(names_from(r#"{"names":[],"error":"no layout in the database"}"#).is_empty());
        assert!(names_from("").is_empty());
        assert_eq!(names_from(r#"{"names":["Base",7,"Media"]}"#), vec!["Base", "", "Media"]);
    }

    #[test]
    fn the_osd_is_asked_with_show_first() {
        let args = osd_args(Path::new("/s/osd.sh"), ICON_LAYER, "Symbols layer", LAYER_MS);
        assert_eq!(
            args,
            vec!["/s/osd.sh", "show", "moonlander-layer", "Symbols layer", "", "", "1200"]
        );
    }

    #[test]
    fn the_status_says_why_there_are_no_layer_popups() {
        let none = BTreeMap::new();
        let one = map(&[("1-4", "Moonlander Mark I")]);
        assert_eq!(status_line(&none, &Err("x".into())), "running: no ZSA keyboard plugged in");
        let with = Ok(Status { keymapp: "1".into(), keyboard: Some(Board::default()) });
        assert!(status_line(&one, &with).ends_with("layers from Keymapp"));
        let without = Ok(Status { keymapp: "1".into(), keyboard: None });
        assert!(status_line(&one, &without).contains("connected to no keyboard"));
        assert!(
            status_line(&one, &Err("Keymapp's API does not answer at /s".into())).contains("/s")
        );
        for line in
            [status_line(&none, &with), status_line(&one, &with), status_line(&one, &without)]
        {
            assert!(line.starts_with("running: "), "{line}");
        }
    }

    #[test]
    fn the_status_is_written_before_the_pid() {
        let dir = tmpdir("claim");
        let order = std::cell::RefCell::new(Vec::new());
        claim(
            &dir,
            |d| {
                order.borrow_mut().push("status");
                write_status_in(d, LOOKING_STATUS);
            },
            |d| {
                assert!(d.join("status").exists(), "the status is there before the pid is");
                order.borrow_mut().push("pid");
                write_pid_in(d);
            },
        );
        assert_eq!(*order.borrow(), vec!["status", "pid"]);
        assert_eq!(fs::read_to_string(dir.join("status")).unwrap(), format!("{LOOKING_STATUS}\n"));
        assert_eq!(
            fs::read_to_string(dir.join("pid")).unwrap(),
            format!("{}\n", std::process::id())
        );
    }
}
