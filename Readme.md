# CDLS

CDLS: A cd+ls alternatives for linux system. Help promptly navigate in file system.

# Usage

Usage: cdls [OPTION]

Options:

        -h, --help                      Help message

Operations in cdls screen:

1. Use arrow button to navigate in directory

        Left arrow              go to parent directory
        Right arrow             go to child directory
        Up arrow                go to previous item
        Down arrow              go to next item

2. Configuration Screen

        c                       Column Display
        s                       Sort by

3. Exit cdls

        Enter button                 Exit cdls and jump to current directory

# Dependencies

Rust Crate:

* ncurses
* log
