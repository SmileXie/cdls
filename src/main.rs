
extern crate ncurses;
extern crate simplelog;
extern crate log;
// extern crate fork;
extern crate chrono;

use std::env;
use std::fs;
use std::os::unix::fs::FileTypeExt;
use std::os::unix::fs::PermissionsExt;
//use nix::sys::signal;
use std::path::PathBuf;
use simplelog::*;
use std::process::{Command, exit};
use std::os::unix::process::CommandExt;
use std::env::set_current_dir;

use chrono::offset::Utc;
use chrono::DateTime;

static COLOR_PAIR_HIGHLIGHT: i16 = 1;
static HELP_STR: &str = "Usage: cdls [OPTION]\n
Options:
\t-h, --help\t\t\tHelp message\n
Operations in cdls screen:
1. Use arrow button to navigate in directory
\tLeft arrow\t\tgo to parent directory
\tRight arrow\t\tgo to child directory
\tUp arrow\t\tgo to previous item
\tDown arrow\t\tgo to next item
2. Enter button\t\t\tExit cdls and jump to current directory
3. toggle column display:
\tt\t\t\tItem type
\tp\t\t\tPermission
\ts\t\t\tSize
\tm\t\t\tModified time";

struct ColumnDisplay {
    item_type: bool,
    permission: bool,
    size: bool,
    mtime: bool,
}

fn get_current_dir_element(cur_path: &PathBuf) -> Vec<PathBuf> {
    let mut children = Vec::new();

    for f in fs::read_dir(cur_path).unwrap() {
        match f {
            Ok(file) => {
                children.push(file.path());
            },
            Err(e) => {
                log::warn!("error: {}", e);
                continue;
            }
        }
    }

    return children;
}

fn get_file_type(path: &PathBuf) -> &str {

    let metadata;

    match fs::symlink_metadata(path) {
        Ok(md) => {
            metadata = md;
        }
        Err(_) => {
            return "NO-PERMISSION"
        }
    }    

    let file_type = metadata.file_type();
    if file_type.is_dir() {
        return "DIR";
    }  else if file_type.is_symlink() {
        return "SYMLINK";
    } else if file_type.is_socket() || file_type.is_fifo() {
        return "FD";
    } else if file_type.is_block_device() || file_type.is_char_device() {
        return "DEV";
    } else if file_type.is_file() {
        return "FILE";
    } else {
        return "UNKNOWN";
    }
}

fn get_file_metadata_element(path: &PathBuf) -> (String, String, String) {

    let metadata;

    match fs::metadata(path) {
        Ok(md) => {
            metadata = md;
        }
        Err(_) => {
            return (String::from("UNKNOWN"), String::from("UNKNOWN"), String::from("UNKNOWN"));
        }
    }

    let permissions = metadata.permissions();
    let mode = permissions.mode();
    let size = metadata.len();

    let mut permission_str = String::from("rwxrwxrwx");
    
    for i in 0..9 {
        if mode & (1 << (8 - i)) == 0 {
            permission_str.replace_range(i..i+1, "-");
        }
    }
    
    // log::debug!("permission string len {}", permission_str.len());

    let size_str = size.to_string(); 

    let modified_time_str = match metadata.modified() {
        Ok(time) => {
            let datetime: DateTime<Utc> = time.into();
            datetime.format("%Y-%m-%d %H:%M:%S").to_string()    
        },
        Err(_) => String::from("UNKNOWN")
    };

    return (permission_str, size_str, modified_time_str);
}

fn help_screen(maxy: i32) {
    ncurses::mv(0, 0);
    ncurses::addstr(HELP_STR);

    ncurses::clrtobot();

    ncurses::attron(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
    ncurses::mvaddstr(maxy - 1, 0, "Press any key to continue");
    ncurses::attroff(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
    
    ncurses::refresh();
}

fn get_item_row_str(col_disp: &ColumnDisplay, file_type: &str, permissions: &str, size: &str, file_name: &str, mtime: &str) -> String {

    let mut row_str:String;

    if col_disp.item_type {
        row_str = format!("{:<8}", file_type);
    } else {
        row_str = String::from("");
    } 

    if col_disp.permission {
        row_str = format!("{}{:<16}", row_str, permissions);
    }

    if col_disp.size {
        row_str = format!("{}{:<16}", row_str, size);
    }

    if col_disp.mtime {
        row_str = format!("{}{:<24}", row_str, mtime);
    }

    row_str = format!("{}{}\n", row_str, file_name);

    return row_str;
}

fn update_dir_screen(basepath: &PathBuf, cursor: usize, start_idx: usize, maxy: i32, col_disp: &ColumnDisplay) 
        -> (Vec<PathBuf>, usize) {
    // todo: display file owner
    // todo: screen height limit, if too small, prompt. 

    ncurses::clear();
    ncurses::mv(0, 0);

    let bar_str = format!("CDLS # {}\n", basepath.to_str().unwrap());
    ncurses::addstr(&bar_str);

    let dir_children = get_current_dir_element(basepath);

    let mut idx = 0;
    for child in &dir_children {
        if idx < start_idx {
            idx += 1;
            continue;
        }

        if start_idx > 0 && idx == start_idx {
            ncurses::addstr("\t...\n");
            idx += 1;
            continue;
        }
        if idx - start_idx == (maxy - 3) as usize {
            ncurses::addstr("\t...\n");
            break;
        }

        if idx == cursor {
            ncurses::attron(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
        // bug: in Xshell alignment doesn't work;
        } 

        let file_path = child.as_path();

        let file_type = get_file_type(child);
        let mut file_name = child.clone().file_name().expect("").to_str().unwrap().to_string();
        
        if file_type.eq("SYMLINK") {
            let sym_link_to = match fs::read_link(file_path) {
                Ok(link_to) => {
                    match link_to.to_str() {
                        Some(link_str) => String::from(link_str),
                        _ => String::from("")
                    }
                },
                Err(_) => String::from("")
            };

            file_name.push_str(" -> ");
            file_name.push_str(&sym_link_to);
        }

        let (permissions, size, mtime) = get_file_metadata_element(child);
        
        let mut row_str = get_item_row_str(col_disp, file_type, &permissions, &size, &file_name, &mtime);

        if idx == cursor {
            row_str.insert_str(0, ">>>>\t");
        } else {
            row_str.insert_str(0, "    \t");
        }

        ncurses::addstr(&row_str);

        if idx == cursor {
            ncurses::attroff(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
        // bug: in Xshell alignment doesn't work;
        }

        idx += 1;
    } 

    ncurses::clrtobot();

    let bt_str = "Arrow Keys: Select item; Enter: Quit cdls and jump to selected item; h: More help";
    ncurses::attron(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
    ncurses::mvaddstr(maxy - 1, 0, bt_str);
    ncurses::attroff(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));

    ncurses::refresh();

    return (dir_children, start_idx);
}

fn print_help() {
    println!("{}", HELP_STR);
}

fn main() {
    
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 {
        print_help();
        exit(0);
    }

    let mut debug_mode = false;
    if args.len() == 2 {
        if args[1] == "--help" || args[1] == "-h" {
            print_help();            
            exit(0);
        } else if args[1] == "--debug" || args[1] == "-d" {
            debug_mode = true;
        } else {
            print_help();
            exit(0);
        }
    }

    if debug_mode {
        match fs::File::create(".cdls.log") {
            Ok(fd) => {
                WriteLogger::init(LevelFilter::Debug, Config::default(), fd).unwrap();
            },
            Err(io_error) => {
                println!("Fail to create log file {}, {}", ".cdls.log", io_error);
                exit(1);
            },
        };  
    }

    let rst = env::current_dir();
    let mut cur_path = match rst {
        Ok(path) => path,
        Err(e) => {
            log::error!("Fail to open current directory. {}", e);
            exit(1);
        }
    };

    ncurses::initscr();
    ncurses::keypad(ncurses::stdscr(), true);
    ncurses::noecho();
    /* Invisible cursor. */
    ncurses::curs_set(ncurses::CURSOR_VISIBILITY::CURSOR_INVISIBLE);

    ncurses::start_color();
    ncurses::init_pair(COLOR_PAIR_HIGHLIGHT, ncurses::COLOR_BLACK, ncurses::COLOR_WHITE);

    let mut cursor: usize = 0;
    let mut start_idx: usize = 0;
    let mut maxy = ncurses::getmaxy(ncurses::stdscr());
    let mut col_disp = ColumnDisplay {item_type: true, permission: true, size: true, mtime: true};
    let mut help_disp = false;
    loop {
        if help_disp {
            help_screen(maxy);
            ncurses::getch(); /* press any key to exit help screen */
            help_disp = false;
            continue;
        } 
        // todo: search mode, search file name
        // todo: order by size / modification time
        let (dir_children, _) = update_dir_screen(&cur_path, cursor, start_idx, maxy, &col_disp);
        let ch = ncurses::getch();
        maxy = ncurses::getmaxy(ncurses::stdscr());
        match ch {
            ncurses::KEY_UP => {
                if cursor > 0 {
                    cursor -= 1;
                    if start_idx > 0 && cursor <= start_idx {
                        start_idx -= 1;
                    }
                }
            },
            ncurses::KEY_DOWN => {
                if cursor < dir_children.len() - 1 {
                    cursor += 1;
                    if cursor - start_idx >= (maxy - 3) as usize {
                        start_idx += 1;
                    }
                }
            },
            ncurses::KEY_LEFT => {
                cur_path.pop();
                cursor = 0;
                start_idx = 0;
            },
            ncurses::KEY_RIGHT => {
                let child = &dir_children[cursor];
                if child.is_dir() {
                    cur_path.push(child.file_name().expect(""));
                    cursor = 0;
                    start_idx = 0;
                }                  
            },
            10 | ncurses::KEY_ENTER => { // enter
                let mut child =  dir_children[cursor].clone();
                if !child.is_dir() {
                    child.pop();
                }
                set_current_dir(&child).unwrap();

                break;
            },
            113 => { /* q */
                log::warn!("q pressed, exit");
                break;
            },
            115 => { /* s */
                col_disp.size = !col_disp.size;
            },
            116 => { /* t */
                col_disp.item_type = !col_disp.item_type;
            },
            112 => { /* p */
                col_disp.permission = !col_disp.permission;
            },
            109 => { /* m */
                col_disp.mtime = !col_disp.mtime;
            },
            104 => { /* h */
                help_disp = true;
            },
            _ => {
                // ncurses::mvaddstr(10, 0, &format!("press {}", ch));
                log::debug!("press {}", ch);
            }
        }
    
    }

    ncurses::echo();
    ncurses::keypad(ncurses::stdscr(), false);
    ncurses::curs_set(ncurses::CURSOR_VISIBILITY::CURSOR_VISIBLE);
    ncurses::endwin();

    // replace current process context with bash
    Command::new("bash").exec();
    // todo: bug: bash recusively call bash
    // fix: use "exec cdls" to start cdls

    exit(0);
}
