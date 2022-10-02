extern crate ncurses;
extern crate simplelog;
extern crate log;
// extern crate fork;
extern crate chrono;
extern crate strum;
extern crate strum_macros;

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
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use std::fmt;

static COLOR_PAIR_HIGHLIGHT: i16 = 1;
static COLOR_PAIR_WIN: i16 = 2;
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
3. Configuration Screen
\tc\t\t\tColumn Display
\ts\t\t\tSort by";

struct ColumnDisplay {
    item_type: bool,
    permission: bool,
    size: bool,
    mtime: bool,
    sortby: SortBy,
}

#[derive(Debug, EnumIter, PartialEq, Eq, PartialOrd, Copy, Clone)]
enum SortBy {
    Filename,
    ItemType,
    Size,
    MTime,
}

impl fmt::Display for SortBy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SortBy::Filename => write!(f, "File Name"),
            SortBy::ItemType => write!(f, "Item Type"),
            SortBy::Size => write!(f, "Size"),
            SortBy::MTime => write!(f, "Modification Time"),
        }
    }
}

impl SortBy {
    fn to_usize(&self) -> usize {
        match self {
            SortBy::Filename => 0,
            SortBy::ItemType => 1,
            SortBy::Size => 2,
            SortBy::MTime => 3,
        }
    }
}

trait PathBufExt {
    fn file_size(&self) -> u64;
}

impl PathBufExt for PathBuf {
    fn file_size(&self) -> u64 {
        match fs::symlink_metadata(self) {
            Ok(md) => {
                return md.len();
            }
            Err(_) => {
                return 0;
            }
        }
    }
}

fn get_current_dir_element(cur_path: &PathBuf, col_disp: &ColumnDisplay) -> Vec<PathBuf> {
    let mut children = Vec::new();

    let read_dir_iter = fs::read_dir(cur_path);
    match read_dir_iter {
        Ok(_) => {},
        Err(e) => {
            log::warn!("read_dir_iter error: {}", e);
            return children;
        }
    }

    for f in read_dir_iter.unwrap() {
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

    match col_disp.sortby {
        SortBy::Filename => children.sort_by(|a, b| a.cmp(&b)),
        SortBy::Size => children.sort_by(|a, b| a.file_size().cmp(&b.file_size())),
        _ => {}, 
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

    match fs::symlink_metadata(path) {
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

fn main_screen_update(basepath: &PathBuf, cursor: usize, start_idx: usize, maxy: i32, col_disp: &ColumnDisplay) 
        -> (Vec<PathBuf>, usize) {
    // todo: display file owner
    // todo: screen height limit, if too small, prompt. 

    ncurses::clear();
    ncurses::mv(0, 0);

    let bar_str = format!("CDLS # {}\n", basepath.to_str().unwrap());
    ncurses::addstr(&bar_str);

    let dir_children = get_current_dir_element(basepath, col_disp);

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

fn column_cfg_screen_update(maxy: i32, col_disp: &ColumnDisplay, selected: usize) {
    ncurses::clear();
    ncurses::mv(0, 0);

    ncurses::addstr("Please Select Columns to Display\n");

    if selected == 0 {
        ncurses::attron(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
    }
    if col_disp.item_type {
        ncurses::addstr("* Item Type\n");
    } else {
        ncurses::addstr("  Item Type\n");
    }
    if selected == 0 {
        ncurses::attroff(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
    }

    if selected == 1 {
        ncurses::attron(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
    }
    if col_disp.permission {
        ncurses::addstr("* Permission\n");
    } else {
        ncurses::addstr("  Permission\n");
    }
    if selected == 1 {
        ncurses::attroff(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
    }

    if selected == 2 {
        ncurses::attron(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
    }
    if col_disp.size {
        ncurses::addstr("* Size\n");
    } else {
        ncurses::addstr("  Size\n");
    }
    if selected == 2 {
        ncurses::attroff(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
    }

    if selected == 3 {
        ncurses::attron(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
    }
    if col_disp.mtime {
        ncurses::addstr("* Modification Time\n");
    } else {
        ncurses::addstr("  Modification Time\n");
    }
    if selected == 3 {
        ncurses::attroff(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
    }

    let bt_str = "Space: Toggle Selection; Enter: Save and Quit";
    ncurses::attron(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
    ncurses::mvaddstr(maxy - 1, 0, bt_str);
    ncurses::attroff(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));

    ncurses::refresh();
}

fn column_cfg(maxy: i32, col_disp: &mut ColumnDisplay) {
   
    let mut selected: usize = 0;

    column_cfg_screen_update(maxy, col_disp, selected);

    loop {
        let ch = ncurses::getch();
        
        match ch {
            32 => { /* space */
                match selected {
                    0 => col_disp.item_type = !col_disp.item_type,
                    1 => col_disp.permission = !col_disp.permission,
                    2 => col_disp.size = !col_disp.size,
                    3 => col_disp.mtime = !col_disp.mtime,
                    _ => {}
                }
                column_cfg_screen_update(maxy, col_disp, selected);
            },
            10 | ncurses::KEY_ENTER => { // enter
                return;
            },
            ncurses::KEY_UP => {
                if selected > 0 {
                    selected -= 1;
                }
                column_cfg_screen_update(maxy, col_disp, selected);
            },
            ncurses::KEY_DOWN => {
                if selected < 3 {
                    selected += 1;
                }
                column_cfg_screen_update(maxy, col_disp, selected);
            },
            _ => {
                column_cfg_screen_update(maxy, col_disp, selected);
            }
        }
    }

}

fn sort_cfg_screen_update(maxy: i32, col_disp: &ColumnDisplay, selected: &SortBy) {
    ncurses::clear();
    ncurses::mv(0, 0);

    ncurses::addstr("Sort tht items by:\n");
    
    for sortby in SortBy::iter() {
        if selected.to_usize() == sortby.to_usize() {
            ncurses::attron(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
        }
        if col_disp.sortby == sortby {
            ncurses::addstr(&format!("* {}\n", sortby.to_string()));
        } else {
            ncurses::addstr(&format!("  {}\n", sortby.to_string()));
        }
        if selected.to_usize() == sortby.to_usize() {
            ncurses::attroff(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
        }
    }

    let bt_str = "Space: Toggle Selection; Enter: Save and Quit";
    ncurses::attron(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
    ncurses::mvaddstr(maxy - 1, 0, bt_str);
    ncurses::attroff(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));

    ncurses::refresh();
}

fn sort_cfg(maxy: i32, col_disp: &mut ColumnDisplay) {
       
    let mut selected: SortBy = SortBy::iter().nth(col_disp.sortby.to_usize()).unwrap();

    sort_cfg_screen_update(maxy, col_disp, &selected);

    loop {
        let ch = ncurses::getch();
        
        match ch {
            32 => { /* space */
                col_disp.sortby = selected;
                sort_cfg_screen_update(maxy, col_disp, &selected);
            },
            10 | ncurses::KEY_ENTER => { // enter
                return;
            },
            ncurses::KEY_UP => {
                if selected > SortBy::Filename {
                    selected = SortBy::iter().nth(selected.to_usize() - 1).unwrap();
                }
                sort_cfg_screen_update(maxy, col_disp, &selected);
            },
            ncurses::KEY_DOWN => {
                if selected < SortBy::MTime {
                    selected = SortBy::iter().nth(selected.to_usize() + 1).unwrap();
                }
                sort_cfg_screen_update(maxy, col_disp, &selected);
            },
            _ => {
                sort_cfg_screen_update(maxy, col_disp, &selected);
            }
        }
    }
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
    ncurses::curs_set(ncurses::CURSOR_VISIBILITY::CURSOR_INVISIBLE);

    ncurses::start_color();
    ncurses::init_pair(COLOR_PAIR_HIGHLIGHT, ncurses::COLOR_BLACK, ncurses::COLOR_WHITE);
    ncurses::init_pair(COLOR_PAIR_WIN, ncurses::COLOR_BLACK, ncurses::COLOR_CYAN);

    let mut cursor: usize = 0;
    let mut start_idx: usize = 0;
    let mut maxy = ncurses::getmaxy(ncurses::stdscr());
    let mut col_disp = ColumnDisplay {item_type: true, permission: true, size: true, mtime: true, sortby: SortBy::Filename};

    loop {
        // todo: search mode, search file name
        // todo: order by size / modification time
        let (dir_children, _) = main_screen_update(&cur_path, cursor, start_idx, maxy, &col_disp);
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
                set_current_dir(&child).unwrap(); // todo: handle error
                break;
            },
            113 => { /* q */
                log::warn!("q pressed, exit");
                break;
            },
            99 => { /* c */
                column_cfg(maxy, &mut col_disp);
            }
            104 => { /* h */
                help_screen(maxy);
                ncurses::getch(); /* press any key to exit help screen */
            },
            115 => { /* s */
                sort_cfg(maxy, &mut col_disp);
            },
            _ => {
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
