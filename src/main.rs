use enigo::*;

extern crate repng;
extern crate scrap;

use scrap::{Capturer, Display};
use std::thread;
use std::time::{Duration, Instant};

use std::sync::mpsc;

fn main() {
    let mut enigo = Enigo::new();
    
    let (tx_interpretation, rx_interpretation) = mpsc::channel();
    let (tx_movement, rx_movement) = mpsc::channel();

    const AIMBOT_VRT: Duration = Duration::from_millis(250);
    const MS_BETWEEN_MOVEMENTS: Duration = Duration::from_millis(10);

    const SCREEN_WIDTH: usize = 1920;
    const SCREEN_HEIGHT: usize = 1080;

    const CENTER_X: i32 = SCREEN_WIDTH as i32 / 2;
    const CENTER_Y: i32 = SCREEN_HEIGHT as i32 / 2;

    // enigo.mouse_down(MouseButton::Left);

    let _thread_capture_stimulus = thread::spawn(move || {
        let display = Display::primary().expect("Couldn't find primary display.");
        let mut capturer = Capturer::new(display).expect("Couldn't begin capture.");

        let stride = SCREEN_WIDTH * 4; // 7680 = BGRA values per horizontal monitor line

        loop {
            let buffer = match capturer.frame() {
                Ok(buffer) => buffer,
                _ => { continue; }
            };

            // It goes Blue, Green, Red, Alpha (BGRA)

            'outer: for y in 0..SCREEN_HEIGHT {
                for x in 0..SCREEN_WIDTH {
                    let i = (stride * y) + (4 * x);

                    if (buffer[i] < 35) && (buffer[i + 1] < 35) && (buffer[i + 2] > 225) {
                        let target_x = x as i32;
                        let target_y = (y + 25) as i32;

                        tx_interpretation.send((target_x, target_y, Instant::now())).unwrap();

                        thread::sleep(Duration::from_millis(247)); // Screenshot takes around 2 milliseconds

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
            let wrapped_index = (((neg_i % len) + len) % len) as usize; // Uh apparently you can't do negative modulus without this 'hack'

            sum += prev_movements[wrapped_index];
        }

        sum
    }

    let thread_movement = thread::spawn(move || {
        let mut last_frame_timestamp = Instant::now();

        let mut target_x = 0;
        let mut target_y = 0;

        let mut prev_n_x_movements: [i32; 100] = [0; 100];
        let mut prev_n_y_movements: [i32; 100] = [0; 100];

        let mut vel_trial_i = 0;

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

            match rx_movement.try_recv() { 
                Ok((new_target_x, new_target_y)) => { 
                    let ms_since_update = Instant::now().duration_since(update_timestamp).as_millis() as f64;
                    let n_count = ms_since_update as i32 / MS_BETWEEN_MOVEMENTS.as_millis() as i32;

                    let player_dist_travel_x = get_prev_n_sum(prev_n_x_movements, vel_trial_i as i32, n_count);
                    let player_dist_travel_y = get_prev_n_sum(prev_n_y_movements, vel_trial_i as i32, n_count);

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
                    
                    println!("pos x, y: {}, {}", absolute_target_x, absolute_target_y);
                    println!("vel x, y: {}, {}", target_x_vel, target_y_vel);

                    target_x = new_target_x;
                    target_y = new_target_y;

                    update_timestamp = Instant::now();
                },
                _ => { }
            }

            // let target_x_in_100ms = target_x + (target_x_vel * 100.0) as i32;
            // let target_y_in_100ms = target_y + (target_y_vel * 100.0) as i32;
            
            let x_speed = (target_x_vel * 2.0) as i32;
            let y_speed = (target_y_vel * 2.0) as i32;

            enigo.mouse_move_relative(x_speed, y_speed);

            prev_n_x_movements[vel_trial_i] = x_speed;
            prev_n_y_movements[vel_trial_i] = y_speed;

            vel_trial_i = (vel_trial_i + 1) % 100;

            last_frame_timestamp = Instant::now();
        }
    });
    
    thread_movement.join();
}
