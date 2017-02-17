=========================
Carbon Bindings for Tokio
=========================

:Status: Beta
:Documentation: http://docs.rs/tk-carbon/


A library to submit data to carbon (graphite). Works in asynchronous main
loop using tokio.

Features:

1. Pluggable name resolution (service discovery)
2. Reconnects to the new host(s) on the fly
3. Connects to multiple hosts and duplicates records if name resolves to
   multiple hosts.


License
=======

Licensed under either of

* Apache License, Version 2.0,
  (./LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license (./LICENSE-MIT or http://opensource.org/licenses/MIT)
  at your option.

Contribution
------------

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

