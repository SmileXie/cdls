
extern crate ncurses;
//use ncurses;

use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

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
                println!("error: {}", e);
                continue;
            }
        }
    }

    return children;
}

fn update_dir_screen(basepath: &PathBuf, cursor: i16) -> Vec<PathBuf> {

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
    ncurses::refresh();

    return dir_children;
}

fn main() {

    let rst = env::current_dir();
    let mut cur_path = match rst {
        Ok(path) => path,
        Err(e) => {
            println!("Fail to open current directory. {}", e);
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

    let mut cursor: i16 = 0;

    loop {
        let dir_children = update_dir_screen(&cur_path, cursor);
        let ch = ncurses::getch();
        match ch {
            ncurses::KEY_UP => {
                cursor -= 1;
                if cursor < 0 {
                    cursor = 0;
                }
            },
            ncurses::KEY_DOWN => cursor += 1,
            ncurses::KEY_LEFT => {
                cur_path.pop();
            },
            ncurses::KEY_RIGHT => {
                let sub_ele = dir_children[cursor as usize].clone();
                if sub_ele.is_dir() {
                    cur_path.push(sub_ele.file_name().expect(""));
                }                
            },

            _ => {
                // ncurses::mvaddstr(10, 0, &format!("press {}", ch));
            }
        }
    
    }

    /* Terminate ncurses. */
    ncurses::endwin();
}
