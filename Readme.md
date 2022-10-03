# CDLS

CDLS: A cd+ls alternatives for linux system. Help promptly navigate in file system.

# Install

In x86-64 architecture,

```
wget https://xs-upload.oss-cn-hangzhou.aliyuncs.com/cdls/release/v0.2/cdls
sudo mv cdls /usr/bin/
sudo chmod +x /usr/bin/cdls
```

# Usage

Usage: 

```
# launch cdls screen
cdls

# display cdls help message
cdls -h
```

Operations in cdls screen:

1. Use arrow button to navigate in directory

        Left arrow              go to parent directory
        Right arrow             go to child directory
        Up arrow                go to previous item
        Down arrow              go to next item

2. Start Configuration Screen

        c                       Column Display
        s                       Sort

        In configuration screen, use `arrow buttons` to navigate in configuration, use `space` to select, and use `q` to confirm.

3. Exit cdls

        Enter button                 Exit cdls and jump to current directory

# Dependencies

* libncurses5

        Install in ubuntu: sudo apt-get install libncurses5-dev


# Bugs

## Leaked Bashs

Once a cdls exited, its context is replaced by a new bash navigating to the targeting directory. The new bash usually inherited from a parent bash.

You can use `exit` to exit the new bash and return to the parent one.

I still have no idea how to fix this. I will appreciate if anyone could help.
