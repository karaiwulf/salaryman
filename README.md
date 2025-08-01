So easy its like having a salaryman manage your services for you.

### Installation

`cargo install salaryman --features "smd"` will install the daemon component.
`cargo install salaryman --features "sm-cli"` will install the cli component.

These are build in **synchronous** rust with multithreading from inside of a `std::thread::scope()`

To build against the protocol used on the UNIX socket, use `cargo add salaryman --features "protocol"`.

### A note on repos
Canonical repo is [svn](https://svn.lesbianunix.dev/viewvc/salaryman/trunk/), the git repos available at [sourcehut](https://git.sr.ht/~spicywolf/salaryman) and [github](https;//github.com/karaiwulf/salaryman) are merely conveniences.
I will still accept issues and patchsets/PRs on either of the git platforms.
