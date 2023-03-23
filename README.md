# "Simple" Web Browser (in Rust)

## WARNING
Parsing can't handle self-closing tags, random characters (even HTML special characters), and to make matters worse, the parsed DOM is handled awfully.
Expect multiple inline nested elements i.e. paragraphs, span, and hyperlinks nested within a div to look broken.

## Description
A very simple (& slow?) TcpStream-based HTTP-only CLI browser.

## A few details...
URI defaults to **localhost:80/**

### Features
- [x] open a web page given a URI and shows the text
- [x] show a list of clickable links (uses keyboard, not that far off)
- [x] download a file regardless of its size
- [ ] download a file in parallel (OPTIONAL)
- [x] follow redirections
- [x] show respective HTTP error messages
- [x] open a web page that is protected by HTTP Basic Authentication
- [ ] can access a web page that is protected behind a login page

### Additional Features
#### User Interface
- [x] basic Terminal UI
- [x] basic command system
- [x] comprehensive UI menu
- [x] loading indicator
- [x] metric for file size
- [x] a much more interactive TUI (currently using ncurses-rs)
- [x] scrolling thru links with keebs

#### Quality Of Life
- [x] incremental auto-naming for nameless files

### Planned Additional Features
- [ ] download progress bar
- [ ] split panel layout between command line, help menu, and logs
