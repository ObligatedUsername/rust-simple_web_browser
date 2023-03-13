# "Simple" Web Browser (in Rust)

## Description
A very simple (& slow) TcpStream-based HTTP-only CLI browser.

## A few details...
URI defaults to **localhost:3000/**

### Features
- [x] open a web page given a URI and shows the text
- [ ] show a list of clickable links
- [x] download a file regardless of its size (doesn't work for binaries)
- [ ] download a file in parallel (OPTIONAL)
- [x] follow redirections
- [x] show respective HTTP error messages
- [x] open a web page that is protected by HTTP Basic Authentication
- [ ] can access a web page that is protected behind a login page

### Additional Features
- [x] basic Terminal UI
- [x] basic command system

### Planned Additional Features
- [ ] loading indicator
- [ ] download progress bar
- [ ] a much more interactive TUI
