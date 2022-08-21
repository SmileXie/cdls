
extern crate ncurses;
extern crate simplelog;
extern crate log;

use std::env;
use std::fs;
//use std::io;
use std::path::PathBuf;
use simplelog::*;
use std::process::Command;
use std::process::exit;
use std::os::unix::process::CommandExt;
use std::env::set_current_dir;

/*
struct GlobalCtrl {
    cur_dir: PathBuf,
    screen_dir: PathBuf,
    screen_cursor: PathBuf
}
*/

static COLOR_PAIR_HIGHLIGHT: i16 = 1;

static COLOR_BACKGROUND: i16 = 16;
static COLOR_FOREGROUND: i16 = 17;

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

fn update_dir_screen(basepath: &PathBuf, cursor: usize, start_idx: usize, maxy: i32) -> (Vec<PathBuf>, usize) {

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
        if idx - start_idx >= (maxy - 2) as usize {
            ncurses::addstr("\t...\n");
            break;
        }

        if idx == cursor {
            ncurses::attron(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
        } 

        let mut displayed_ele = child.clone().file_name().expect("").to_str().unwrap().to_string();
        displayed_ele.insert(0, '\t');
        displayed_ele.push('\n');
        ncurses::addstr(&displayed_ele);

        if idx == cursor {
            ncurses::attroff(ncurses::COLOR_PAIR(COLOR_PAIR_HIGHLIGHT));
        }

        idx += 1;
    } 

    ncurses::clrtobot();

        //ncurses::mvaddstr(maxy, 0, "BOTTOM LINE PROMPT");
    
    ncurses::refresh();

    return (dir_children, start_idx);
}

fn print_help() {
    println!("Usage: cdls [OPTION]\n
    Options:
    \t-h, --help\t\t\tHelp message\n
    Operations in cdls screen:
    1. Use arrow button to navigate in directory
    \tLeft arrow\t\t\tgo to parent directory
    \tRight arrow\t\t\tgo to child directory
    \tUp arrow\t\t\tgo to previous item
    \tDown arrow\t\t\tgo to next item
    2. Enter button\t\t\tExit cdls and jump to current directory");
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
        }
        if args[1] == "--debug" {
            debug_mode = true;
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
/*
    let mut g = GlobalCtrl{
        cur_dir: cur_path.clone(), 
        screen_dir: cur_path.clone(), 
        screen_cursor: cur_path.clone()
    };
*/
    ncurses::initscr();
    ncurses::keypad(ncurses::stdscr(), true);
    ncurses::noecho();
    /* Invisible cursor. */
    ncurses::curs_set(ncurses::CURSOR_VISIBILITY::CURSOR_INVISIBLE);

    ncurses::start_color();
    ncurses::init_color(COLOR_BACKGROUND, 0x39 * 4, 0x3e * 4, 0x46 * 4);
    ncurses::init_color(COLOR_FOREGROUND, 0xee * 4, 0xee * 4, 0xee * 4);
    ncurses::init_pair(COLOR_PAIR_HIGHLIGHT, COLOR_BACKGROUND, COLOR_FOREGROUND);

    let mut cursor: usize = 0;
    let mut start_idx: usize = 0;
    let mut maxy = ncurses::getmaxy(ncurses::stdscr());

    loop {
        let (dir_children, _) = update_dir_screen(&cur_path, cursor, start_idx, maxy);
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
                    if cursor - start_idx >= (maxy - 2) as usize {
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
            _ => {
                // ncurses::mvaddstr(10, 0, &format!("press {}", ch));
                log::debug!("press {}", ch);
            }
        }
    
    }

    ncurses::echo();
    ncurses::keypad(ncurses::stdscr(), false);
    ncurses::curs_set(ncurses::CURSOR_VISIBILITY::CURSOR_VISIBLE);
    /* Terminate ncurses. */
    ncurses::endwin();

    // todo kill parent bash process
    Command::new("bash").exec();

    exit(0);
}
