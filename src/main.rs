extern crate ncurses;
extern crate simplelog;
extern crate log;
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

use chrono::offset::{Utc, Local};
use chrono::{DateTime, NaiveDateTime};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use std::fmt;
use rust_fuzzy_search::fuzzy_compare;
use encoding8::ascii::is_printable;

static COLOR_PAIR_HIGHLIGHT: i16 = 1;
static COLOR_PAIR_WIN: i16 = 2;
static HELP_STR: &str = "Usage: cdls [OPTION]\n
Options:
\t-h, --help\t\t\tHelp message\n
Operations in cdls screen:
1. Use arrow button to navigate in directory
\tLeft arrow\t\tGo to parent directory
\tRight arrow\t\tGo to child directory
\tUp arrow\t\tGo to previous item
\tDown arrow\t\tGo to next item
2. Enter button\t\t\tExit cdls and jump to current directory
3. Configuration Screen
\tc\t\t\tColumn Display
\ts\t\t\tSort by
\tIn configuration screen, use `arrow buttons` to navigate in configuration, use `space` to select, and use `q` to confirm.
4. Search Mode
\tf\t\t\tStart search mode
\tIn search mode, type the keywowrds, the item with better matching will rank higher. Use `up/down` to select items, use `enter` to exit search mode.
";

struct CdlsConfig {
    item_type: bool,
    permission: bool,
    size: bool,
    mtime: bool,
    sortby: SortBy,
    search_mode: bool,
    search_string: String,
}

struct CdlsCurPosition {
    cur_dir: PathBuf,
    cur_item: PathBuf,
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
    fn file_modified_time(&self) -> DateTime<Utc>;
    fn file_type(&self) -> &str;
    fn fuzzy_search_score(&self, search_str: &str) -> f32;
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

    fn file_modified_time(&self) -> DateTime<Utc> {
        match fs::symlink_metadata(self) {
            Ok(md) => {
                match md.modified() {
                    Ok(time) => {
                        return time.into();
                    }
                    Err(_) => {
                        return DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc);
                    }
                }
            }
            Err(_) => {
                return DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc);
            }
        }
    }

    fn file_type(&self) -> &str {
        let metadata;

        match fs::symlink_metadata(self) {
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

    fn fuzzy_search_score(&self, search_str: &str) -> f32 {
        let file_name = match self.file_name() {
            Some(name) => {
                match name.to_str() {
                    Some(name_str) => {
                        name_str
                    },
                    None => {
                        ""
                    }
                }
            },
            None => {
                ""
            }
        };
        return fuzzy_compare(search_str, file_name);
    }
}


trait I32Ext {
    fn within_u8_range(&self) -> bool;
    fn to_char(&self) -> char;
    fn to_u8(&self) -> u8;
}

impl I32Ext for i32 {
    fn within_u8_range(&self) -> bool {
        return *self >= 0 && *self <= 255;
    }
    fn to_char(&self) -> char {
        let tmp: u8;
        if self < &0 {
            tmp = 0;
        } else if self > &255 {
            tmp = 255;
        } else {
            tmp = *self as u8;
        }

        return tmp as char;
    }
    fn to_u8(&self) -> u8 {
        let tmp: u8;
        if self < &0 {
            tmp = 0;
        } else if self > &255 {
            tmp = 255;
        } else {
            tmp = *self as u8;
        }

        return tmp;
    }
}

fn get_current_dir_element(cur_position: &mut CdlsCurPosition, cdls_cfg: &CdlsConfig) -> Vec<PathBuf> {
    let mut children = Vec::new();

    let read_dir_iter = fs::read_dir(cur_position.cur_dir.clone());
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

    if cdls_cfg.search_mode {
        children.sort_by(
            |a, b| 
            b.fuzzy_search_score(&cdls_cfg.search_string)
            .total_cmp(&a.fuzzy_search_score(&cdls_cfg.search_string))
        )
    } else {
        match cdls_cfg.sortby {
            SortBy::Filename => children.sort_by(|a, b| a.cmp(&b)),
            SortBy::Size => children.sort_by(|a, b| a.file_size().cmp(&b.file_size())),
            SortBy::MTime => children.sort_by(|a, b| a.file_modified_time().cmp(&b.file_modified_time())),
            SortBy::ItemType => children.sort_by(|a, b| a.file_type().cmp(&b.file_type())),
            //_ => {}, 
        }
    }

    if children.len() >= 1 && cur_position.cur_dir == cur_position.cur_item {
        // cur_item not set. set it to the first item
        cur_position.cur_item = children[0].clone();
        log::warn!("set current postion: {}", cur_position.cur_item.display());
    }

    return children;
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
    
    let size_str = size.to_string(); 

    let modified_time_str = match metadata.modified() {
        Ok(time) => {
            let datetime: DateTime<Local> = time.into();
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

fn get_item_row_str(cdls_cfg: &CdlsConfig, file_type: &str, permissions: &str, size: &str, file_name: &str, mtime: &str) -> String {

    let mut row_str:String;

    if cdls_cfg.item_type {
        row_str = format!("{:<8}", file_type);
    } else {
        row_str = String::from("");
    } 

    if cdls_cfg.permission {
        row_str = format!("{}{:<16}", row_str, permissions);
    }

    if cdls_cfg.size {
        row_str = format!("{}{:<16}", row_str, size);
    }

    if cdls_cfg.mtime {
        row_str = format!("{}{:<24}", row_str, mtime);
    }

    row_str = format!("{}{}\n", row_str, file_name);

    return row_str;
}

fn main_screen_update(cur_position: &mut CdlsCurPosition, maxy: i32, cdls_cfg: &CdlsConfig) 
        -> (Vec<PathBuf>, usize) {
    // todo: display file owner
    // todo: screen height limit, if too small, prompt.  maxy < 3

    ncurses::clear();
    ncurses::mv(0, 0);

    let bar_str = format!("CDLS # {}\n", cur_position.cur_dir.to_str().unwrap());
    ncurses::addstr(&bar_str);

    let dir_children = get_current_dir_element(cur_position, cdls_cfg);

    log::info!("cur item: {}", cur_position.cur_item.display());

    let cursor = match dir_children.iter().position(|x| *x == cur_position.cur_item) {
        Some(pos) => pos,
        None => 0,
    };

    let start_idx = if cursor as i32 - maxy as i32 + 4 < 0 {
        0
    } else {
        (cursor as i32 - maxy as i32 + 4) as usize
    };   

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

        let file_type = child.file_type();
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
        
        let mut row_str = get_item_row_str(cdls_cfg, file_type, &permissions, &size, &file_name, &mtime);

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

    // ncurses::clrtobot();

    let bt_str: String;
    if cdls_cfg.search_mode {
        bt_str = format!("Search string:{} \tEnter: Exit search mode", cdls_cfg.search_string);
    } else {
        bt_str = String::from("Arrow Keys: Select item; Enter: Quit cdls and jump to selected item; h: More help");
    };
    
    ncurses::attron(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
    ncurses::mvaddstr(maxy - 1, 0, &bt_str);
    ncurses::attroff(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));

    ncurses::refresh();

    return (dir_children, cursor);
}

fn print_help() {
    println!("{}", HELP_STR);
}

fn column_cfg_screen_update(maxy: i32, cdls_cfg: &CdlsConfig, selected: usize) {
    ncurses::clear();
    ncurses::mv(0, 0);

    ncurses::addstr("Please Select Columns to Display\n");

    if selected == 0 {
        ncurses::attron(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
    }
    if cdls_cfg.item_type {
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
    if cdls_cfg.permission {
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
    if cdls_cfg.size {
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
    if cdls_cfg.mtime {
        ncurses::addstr("* Modification Time\n");
    } else {
        ncurses::addstr("  Modification Time\n");
    }
    if selected == 3 {
        ncurses::attroff(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
    }

    let bt_str = "Space: Toggle Selection; q: Save and Quit";
    ncurses::attron(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
    ncurses::mvaddstr(maxy - 1, 0, bt_str);
    ncurses::attroff(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));

    ncurses::refresh();
}

fn column_cfg(maxy: i32, cdls_cfg: &mut CdlsConfig) {
   
    let mut selected: usize = 0;

    column_cfg_screen_update(maxy, cdls_cfg, selected);

    loop {
        let ch = ncurses::getch();
        
        match ch {
            32 => { /* space */
                match selected {
                    0 => cdls_cfg.item_type = !cdls_cfg.item_type,
                    1 => cdls_cfg.permission = !cdls_cfg.permission,
                    2 => cdls_cfg.size = !cdls_cfg.size,
                    3 => cdls_cfg.mtime = !cdls_cfg.mtime,
                    _ => {}
                }
                column_cfg_screen_update(maxy, cdls_cfg, selected);
            },
            113 => { // q
                return;
            },
            ncurses::KEY_UP => {
                if selected > 0 {
                    selected -= 1;
                }
                column_cfg_screen_update(maxy, cdls_cfg, selected);
            },
            ncurses::KEY_DOWN => {
                if selected < 3 {
                    selected += 1;
                }
                column_cfg_screen_update(maxy, cdls_cfg, selected);
            },
            _ => {
                column_cfg_screen_update(maxy, cdls_cfg, selected);
            }
        }
    }

}

fn sort_cfg_screen_update(maxy: i32, cdls_cfg: &CdlsConfig, selected: &SortBy) {
    ncurses::clear();
    ncurses::mv(0, 0);

    ncurses::addstr("Sort tht items by:\n");
    
    for sortby in SortBy::iter() {
        if selected.to_usize() == sortby.to_usize() {
            ncurses::attron(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
        }
        if cdls_cfg.sortby == sortby {
            ncurses::addstr(&format!("* {}\n", sortby.to_string()));
        } else {
            ncurses::addstr(&format!("  {}\n", sortby.to_string()));
        }
        if selected.to_usize() == sortby.to_usize() {
            ncurses::attroff(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
        }
    }

    let bt_str = "Space: Toggle Selection; q: Save and Quit";
    ncurses::attron(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
    ncurses::mvaddstr(maxy - 1, 0, bt_str);
    ncurses::attroff(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));

    ncurses::refresh();
}

fn sort_cfg(maxy: i32, cdls_cfg: &mut CdlsConfig) {
       
    let mut selected: SortBy = SortBy::iter().nth(cdls_cfg.sortby.to_usize()).unwrap();

    sort_cfg_screen_update(maxy, cdls_cfg, &selected);

    loop {
        let ch = ncurses::getch();
        
        match ch {
            32 => { /* space */
                cdls_cfg.sortby = selected;
                sort_cfg_screen_update(maxy, cdls_cfg, &selected);
            },
            113 => { // q
                return;
            },
            ncurses::KEY_UP => {
                if selected > SortBy::Filename {
                    selected = SortBy::iter().nth(selected.to_usize() - 1).unwrap();
                }
                sort_cfg_screen_update(maxy, cdls_cfg, &selected);
            },
            ncurses::KEY_DOWN => {
                if selected < SortBy::MTime {
                    selected = SortBy::iter().nth(selected.to_usize() + 1).unwrap();
                }
                sort_cfg_screen_update(maxy, cdls_cfg, &selected);
            },
            _ => {
                sort_cfg_screen_update(maxy, cdls_cfg, &selected);
            }
        }
    }
}

fn search_mode(cur_position: &mut CdlsCurPosition, maxy: i32, cdls_cfg: &mut CdlsConfig) {

    cdls_cfg.search_mode = true;

    while cdls_cfg.search_mode {
        let (dir_children, cursor) = main_screen_update(cur_position, maxy, &cdls_cfg);

        let ch = ncurses::getch();
        log::debug!("press {}", ch);

        if ch.within_u8_range() && is_printable(ch.to_u8())
                || ch == ncurses::KEY_BACKSPACE 
                || ch == 8 /* backspace */ {
            // reset cursor
            cur_position.cur_item = cur_position.cur_dir.clone();

            if ch == ncurses::KEY_BACKSPACE || ch == 8 {
                cdls_cfg.search_string.pop();
            } else {
                cdls_cfg.search_string.push(ch.to_char());
            }
        }

        match ch {
            ncurses::KEY_UP => {
                if cursor > 0 {
                    cur_position.cur_item = dir_children[cursor - 1].clone();
                }
            },
            ncurses::KEY_DOWN => {
                if cursor < dir_children.len() - 1 {
                    cur_position.cur_item = dir_children[cursor + 1].clone();
                }
            },
            10 | ncurses::KEY_ENTER => { // enter
                // exit search mode
                cdls_cfg.search_mode = false;
                cdls_cfg.search_string.clear();                    
                break;     
            },
            _ => {
                continue;   
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
    let cur_path = match rst {
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

    let mut maxy = ncurses::getmaxy(ncurses::stdscr());
    let mut cdls_cfg = CdlsConfig {
        item_type: true, 
        permission: true, 
        size: true, 
        mtime: true, 
        sortby: SortBy::Filename,
        search_mode: false,
        search_string: String::new(),
    };

    let mut cur_position = CdlsCurPosition {
        cur_dir: cur_path.clone(),
        cur_item: cur_path.clone(),
    };
    
    loop {
        let (dir_children, cursor) = main_screen_update(&mut cur_position, maxy, &cdls_cfg);
        maxy = ncurses::getmaxy(ncurses::stdscr());
        let ch = ncurses::getch();
        log::debug!("press {}", ch);
        log::debug!("cursor {}", cursor);
        log::debug!("dir_children len {}", dir_children.len());
        match ch {
            ncurses::KEY_UP => {
                if cursor > 0 && dir_children.len() > 1  {
                    cur_position.cur_item = dir_children[cursor - 1].clone();
                }
            },
            ncurses::KEY_DOWN => {
                if dir_children.len() >= 1 && cursor < dir_children.len() - 1 {
                    cur_position.cur_item = dir_children[cursor + 1].clone();
                }
            },
            ncurses::KEY_LEFT => {
                cur_position.cur_dir.pop();
                cur_position.cur_item = cur_position.cur_dir.clone();
            },
            ncurses::KEY_RIGHT => {
                let child = &dir_children[cursor];
                if child.is_dir() {
                    cur_position.cur_dir.push(child.file_name().expect(""));
                    cur_position.cur_item = cur_position.cur_dir.clone();
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
                column_cfg(maxy, &mut cdls_cfg);
            }
            102 => { /* f */
                search_mode(&mut cur_position, maxy, &mut cdls_cfg);
            }
            104 => { /* h */
                help_screen(maxy);
                ncurses::getch(); /* press any key to exit help screen */
            },
            115 => { /* s */
                sort_cfg(maxy, &mut cdls_cfg);
            },
            _ => {
                continue;   
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
