extern crate nix;

use nix::fcntl::OFlag;
use std::str;


fn main() {
    let fd = nix::fcntl::open("/dev/kmsg", OFlag::O_RDONLY | OFlag::O_NONBLOCK,
                              nix::sys::stat::Mode::empty()).unwrap();

    loop {
        let mut buff = [0u8; 8193];
        match nix::unistd::read(fd, &mut buff) {
            Ok(l) => l,
            Err(e) => match e {
                nix::Error::Sys(errno) => {
                    println!("errno {}", errno);
                    break;
                }
                _ => 0usize,
            }
        };

        println!("{}", str::from_utf8(&buff).unwrap());
    }
    nix::unistd::close(fd);
}
