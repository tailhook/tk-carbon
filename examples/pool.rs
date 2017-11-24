extern crate abstract_ns;
extern crate futures;
extern crate futures_cpupool;
extern crate ns_std_threaded;
extern crate regex;
extern crate tk_carbon;
extern crate tk_easyloop;
extern crate tokio_core;
extern crate env_logger;

use std::env::args;
use std::io::{self, BufRead};
use std::thread;

use abstract_ns::Resolver;
use regex::Regex;
use tk_carbon::{Carbon, Config};


fn main() {
    env_logger::init().expect("init logging");
    let name = args().skip(1).next().unwrap_or("localhost:2003".to_string());
    let (carbon, init) = Carbon::new(&Config::new().done());
    // run io in thread because stdin is not officially supported in
    // mio/tokio yet
    thread::spawn(|| {
        tk_easyloop::run_forever(move || {
            let resolver = ns_std_threaded::ThreadedResolver::new(
                futures_cpupool::CpuPool::new(1));
            init.connect_to(resolver.subscribe(&name), &tk_easyloop::handle());
            Ok::<(), ()>(())
        }).unwrap();
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
