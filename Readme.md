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

        In configuration screen, use arrow button to navigate in configuration, use space to select, and use enter to confirm.

3. Exit cdls

        Enter button                 Exit cdls and jump to current directory

# Bugs

## Leaked Bashs

Once a cdls exited, its context is replaced by a new bash navigating to the targeting directory. The new bash usually inherited from a parent bash.

You can use `exit` to exit the new bash and return to the parent one.

I still have no idea how to fix this. I will appreciate it if you can help me.
