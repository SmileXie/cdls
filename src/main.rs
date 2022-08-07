
extern crate ncurses;
extern crate simplelog;
extern crate log;

use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use simplelog::*;

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

fn update_dir_screen(basepath: &PathBuf, cursor: usize) -> Vec<PathBuf> {

    ncurses::mv(0, 0);

    let bar_str = format!("CDLS # {} ...\n", basepath.to_str().unwrap());
    ncurses::addstr(&bar_str);

    let dir_children = get_current_dir_element(basepath);

    let mut idx = 0;
    for child in &dir_children {
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

    //let maxy = ncurses::getmaxy(ncurses::stdscr());
    //ncurses::mvaddstr(maxy, 0, "BOTTOM LINE PROMPT");
    
    ncurses::refresh();

    return dir_children;
}

fn main() {

    // todo!("Create log file only in debug mode, which is passed as an argument");

    match fs::File::create(".cdls.log") {
        Ok(fd) => {
            WriteLogger::init(LevelFilter::Debug, Config::default(), fd);
        },
        Err(io_error) => {
            println!("Fail to create log file {}, {}", ".cdls.log", io_error);
            return;
        },
    };  

    let rst = env::current_dir();
    let mut cur_path = match rst {
        Ok(path) => path,
        Err(e) => {
            log::error!("Fail to open current directory. {}", e);
            return;
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

    loop {
        let dir_children = update_dir_screen(&cur_path, cursor);
        let ch = ncurses::getch();
        match ch {
            ncurses::KEY_UP => {
                if cursor > 0 {
                    cursor -= 1;
                }
            },
            ncurses::KEY_DOWN => {
                if cursor < dir_children.len() - 1 {
                    cursor += 1;
                }
            },
            ncurses::KEY_LEFT => {
                cur_path.pop();
            },
            ncurses::KEY_RIGHT => {
                let child = &dir_children[cursor];
                if child.is_dir() {
                    cur_path.push(child.file_name().expect(""));
                }                
            },
            ncurses::KEY_ENTER => {

            },
            113 => { /* q */
                log::debug!("q pressed, exit");
                break;
            },
            _ => {
                // ncurses::mvaddstr(10, 0, &format!("press {}", ch));
                log::debug!("press {}", ch);
            }
        }
    
    }

    /* Terminate ncurses. */
    ncurses::endwin();
}
