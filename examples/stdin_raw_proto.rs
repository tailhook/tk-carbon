extern crate futures;
extern crate regex;
extern crate tk_easyloop;
extern crate tk_carbon;
extern crate tokio_core;

use std::io::{self, BufRead};
use std::net::SocketAddr;
use std::thread;

use futures::Future;
use regex::Regex;
use tk_carbon::{Carbon, Config};
use tokio_core::net::TcpStream;


fn main() {
    let (carbon, init) = Carbon::new(&Config::new().done());
    // run io in thread because stdin is not officially supported in
    // mio/tokio yet
    thread::spawn(|| {
        tk_easyloop::run(|| {
            TcpStream::connect(&SocketAddr::new("127.0.0.1".parse().unwrap(),
                                                2003),
                               &tk_easyloop::handle())
            .map_err(|e| println!("Carbon error: {}", e))
            .and_then(|sock| {
                init.from_connection(sock, &tk_easyloop::handle())
            })
        })
    });
    let regex = Regex::new(r"^([a-zA-Z0-9\.-]+)(?:\s+(\d+))$").unwrap();
    println!("Enter `metric.name 134`:");

    for line in io::BufReader::new(io::stdin()).lines() {
        let line = match line {
            Ok(x) => x,
            Err(_) => return,
        };
        if let Some(capt) = regex.captures(&line) {
            let name = capt.get(1).unwrap().as_str();
            let value: i64 = capt.get(2).unwrap().as_str().parse().unwrap();
            carbon.add_value(format_args!("test.{}", name), value);
            println!("Metric {:?} value {:?}",
                format_args!("test.{}", name),
                value);
        } else {
            println!("Invalid format. Use `metric.name 1235`.");
        }
    }
}
