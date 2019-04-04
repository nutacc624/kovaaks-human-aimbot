use enigo::*;

extern crate repng;
extern crate scrap;

use scrap::{Capturer, Display};
use std::io::ErrorKind::WouldBlock;
use std::thread;
use std::time::{Duration, Instant};

use std::sync::mpsc;

fn main() {
    let mut enigo = Enigo::new();
    
    let (tx_interpretation, rx_interpretation) = mpsc::channel();
    let (tx_movement, rx_movement) = mpsc::channel();

    const AIMBOT_VRT: Duration = Duration::from_millis(250);

    const ONE_FRAME: Duration = Duration::from_millis(16);

    const CENTER_X: i32 = 960;
    const CENTER_Y: i32 = 540;

    enigo.mouse_move_to(1920 / 2, 1080 / 2);
    thread::sleep(ONE_FRAME);
    enigo.mouse_down(MouseButton::Left);
    thread::sleep(ONE_FRAME);

    let _thread_capture_stimulus = thread::spawn(move || {
        let display = Display::primary().expect("Couldn't find primary display.");
        let mut capturer = Capturer::new(display).expect("Couldn't begin capture.");
        let (w, h) = (capturer.width(), capturer.height());

        let stride = w * 4; // 7680 = rgba values per horizontal monitor line
        let padding = 500;

        let mut found = false;

        loop {
            let buffer = match capturer.frame() {
                Ok(buffer) => buffer,
                Err(error) => {
                    if error.kind() == WouldBlock {
                        continue;
                    } else {
                        panic!("Error: {}", error);
                    }
                }
            };

            // it goes blue, green, red, alpha (BGRA)

            let y_lower_bound = if found { (h / 2) - padding } else { 0 };
            let y_upper_bound = if found { (h / 2) + padding } else { h };

            let x_lower_bound = if found { (w / 2) as usize - padding } else { 0 };
            let x_upper_bound = if found { (w / 2) as usize + padding } else { w };

            'outer: for y in y_lower_bound..y_upper_bound {
                for x in x_lower_bound..x_upper_bound {
                    let i = (stride * y) + (4 * x);

                    if (buffer[i] < 35) && (buffer[i + 1] < 35) && (buffer[i + 2] > 225) {
                        let target_x = x as i32;
                        let target_y = (y + 25) as i32;

                        tx_interpretation.send((target_x, target_y, Instant::now())).unwrap();

                        thread::sleep(Duration::from_millis(247));

                        found = true;

                        break 'outer;
                    }
                }
            }
        }
    });

    let _thread_interpretation = thread::spawn(move || {
        loop {
            match rx_interpretation.recv() {
                Ok((target_x, target_y, timestamp)) => { 
                    let time_since_stimulus = Instant::now().duration_since(timestamp);

                    if time_since_stimulus < AIMBOT_VRT { 
                        thread::sleep(AIMBOT_VRT - time_since_stimulus);
                    }

                    tx_movement.send((target_x, target_y)).unwrap();
                },
                _ => { continue; }
            }
        }
    });

    fn get_prev_n_sum(prev_movements: [i32; 100], start_index: i32, n: i32) -> i32 {
        let mut sum = 0;

        for i in 0..n {
            let neg_i = start_index - i;
            let len: i32 = prev_movements.len() as i32;
            let wrapped_index = (((neg_i % len) + len) % len) as usize;

            sum += prev_movements[wrapped_index];
        }

        sum
    }

    let thread_movement = thread::spawn(move || {
        let mut last_frame_timestamp = Instant::now();

        let mut target_x = 0;
        let mut target_y = 0;

        const MS_BETWEEN_MOVEMENTS: Duration = Duration::from_millis(10);

        let trial_count = 10;

        let mut vel_trial_i = 0;

        let mut prev_n_x_movements: [i32; 100] = [0; 100];
        let mut prev_n_y_movements: [i32; 100] = [0; 100];
        let mut movement_x_i: usize = 0;
        let mut movement_y_i: usize = 0;

        let mut absolute_target_x = 0;
        let mut absolute_target_y = 0;

        let mut target_x_vel: f64 = 0.0;
        let mut target_y_vel: f64 = 0.0;

        let mut update_timestamp = Instant::now();

        loop {
            let ms_delta = Instant::now().duration_since(last_frame_timestamp);

            if ms_delta < MS_BETWEEN_MOVEMENTS { 
                thread::sleep(MS_BETWEEN_MOVEMENTS - ms_delta);
            }

            let x_delta = target_x - CENTER_X;
            let y_delta = target_y - CENTER_Y;

            match rx_movement.try_recv() { 
                Ok((new_target_x, new_target_y)) => { 
                    let ms_since_update = Instant::now().duration_since(update_timestamp).as_millis() as f64;
                    let n_count = ms_since_update as i32 / 10;

                    let player_dist_travel_x = get_prev_n_sum(prev_n_x_movements, movement_x_i as i32, n_count);
                    let player_dist_travel_y = get_prev_n_sum(prev_n_y_movements, movement_y_i as i32, n_count);

                    let old_x_delta = target_x - CENTER_X;
                    let old_y_delta = target_y - CENTER_Y;

                    let new_x_delta = new_target_x - CENTER_X;
                    let new_y_delta = new_target_y - CENTER_Y;

                    let target_x_change = player_dist_travel_x + old_x_delta + new_x_delta;
                    let target_y_change = player_dist_travel_y + old_y_delta + new_y_delta;

                    target_x_vel = target_x_change as f64 / ms_since_update;
                    target_y_vel = target_y_change as f64 / ms_since_update;

                    absolute_target_x += target_x_change;
                    absolute_target_y += target_y_change;

                    target_x = new_target_x;
                    target_y = new_target_y;

                    update_timestamp = Instant::now();
                },
                _ => { }
            }

            // let target_x_vel: f64 = (absolute_target_x - prev_absolute_target_x) as f64 / (250.0);
            // let target_y_vel: f64 = (absolute_target_y - prev_absolute_target_y) as f64 / (250.0);

            // let target_x_in_100ms = target_x + (target_x_vel * 100.0) as i32;
            // let target_y_in_100ms = target_y + (target_y_vel * 100.0) as i32;

            vel_trial_i = (vel_trial_i + 1) % (trial_count - 1);

            let guessed_target_x = target_x + (target_x_vel * 250.0) as i32;
            let guessed_target_y = target_y + (target_y_vel * 250.0) as i32;

            let guess_x_delta = guessed_target_x - CENTER_X;
            let guess_y_delta = guessed_target_y - CENTER_Y;

            /*
            if guess_x_delta > 50 || guess_x_delta < -50 || guess_y_delta > 50 || guess_y_delta < -50 {
                enigo.mouse_move_relative(guess_x_delta / 5, guess_y_delta / 5);

                prev_n_x_movements[movement_x_i] = guess_x_delta / 5;
                prev_n_y_movements[movement_y_i] = guess_y_delta / 5;

                println!("{}, {}", guess_x_delta, guess_y_delta);

                target_x = CENTER_X;
                target_y = CENTER_Y;
            } else {
            */

                let x_speed = (target_x_vel * 10.0) as i32;
                let y_speed = (target_y_vel * 10.0) as i32;

                // enigo.mouse_move_relative(x_speed, y_speed);

                // println!("{}, {}", x_speed, y_speed);

                // prev_n_x_movements[movement_x_i] = x_speed;
                // prev_n_y_movements[movement_y_i] = y_speed;
            //}

            movement_x_i = (movement_x_i + 1) % 100;
            movement_y_i = (movement_y_i + 1) % 100;

            last_frame_timestamp = Instant::now();
        }
    });
    
    thread_movement.join();
}
