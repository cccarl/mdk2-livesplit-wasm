use asr::{Process, watcher::{Pair}, time::Duration};
use spinning_top::{Spinlock, const_spinlock};

const MAIN_MODULE: &str = "mdk2Main.exe";
const LOADING_PATH: [u64; 1] = [0x1D1224];
const LEVEL_PATH: [u64; 1] = [0xBB724];
const SUBLEVEL_PATH: [u64; 1] = [0xBBA8C];
const MUSIC_PATH: [u64; 1] = [0xBC364];


fn update_pair_i32(variable_name: &str, new_value: i32, pair: &mut Pair<i32>) {
    asr::timer::set_variable(variable_name, &format!("{new_value}"));
    pair.old = pair.current;
    pair.current = new_value;
}


struct MemoryValues {
    loading: Pair<i32>,
    level: Pair<i32>,
    sublevel: Pair<i32>,
    music: Pair<i32>,
}

struct State {
    started_up: bool,
    main_process: Option<Process>,
    values: MemoryValues,
}

impl State {

    fn refresh_mem_values(&mut self) -> Result<&str, &str> {

        let main_module_addr = match &self.main_process {
            Some(info) => match info.get_module_address(MAIN_MODULE) {
                Ok(address) => address,
                Err(_) => return Err("Could not get module address when refreshing memory values.")
            },
            None => return Err("Process info is not initialized.")
        };

        let process = self.main_process.as_ref().unwrap();

        // insert read int calls here
        if let Ok(value) = process.read_pointer_path64::<i32>(main_module_addr.0, &LOADING_PATH) {
            update_pair_i32("Loading", value, &mut self.values.loading);
        }

        if let Ok(value) = process.read_pointer_path64::<i32>(main_module_addr.0, &LEVEL_PATH) {
            update_pair_i32("Level", value, &mut self.values.level);
        }

        if let Ok(value) = process.read_pointer_path64::<i32>(main_module_addr.0, &SUBLEVEL_PATH) {
            update_pair_i32("Sublevel", value, &mut self.values.sublevel);
        }

        if let Ok(value) = process.read_pointer_path64::<i32>(main_module_addr.0, &MUSIC_PATH) {
            update_pair_i32("Music", value, &mut self.values.music);
        }

        Ok("Success")
    }

    fn startup(&mut self) {
        asr::set_tick_rate(10.0);
        self.started_up = true;
    }

    fn init(&mut self) {
        asr::set_tick_rate(120.0);
    }

    fn update(&mut self) {

        if !self.started_up {
            self.startup();
        }

        if self.main_process.is_none() {
            self.main_process = Process::attach(MAIN_MODULE);
            if self.main_process.is_some() {
                self.init();
            }
            // early return to never work with a None process
            return;
        }

        // if game is closed detatch and look for the game again
        if !self.main_process.as_ref().unwrap().is_open() {
            asr::set_tick_rate(10.0);
            self.main_process = None;
            return;
        }

        // update memory values using the watchers
        if self.refresh_mem_values().is_err() {
            return;
        }

        // start condition
        if self.values.level.current == 1 && self.values.sublevel.current == 9 && self.values.loading.current == 1 {
            asr::timer::set_game_time(Duration::milliseconds(1231));
            asr::timer::start();
        }

        // pause game time (loadless timer)
        // TODO: does the asr debugger not support loadless timer or what
        if self.values.loading.current == 1 {
            asr::timer::pause_game_time();
        } else {
            asr::timer::resume_game_time();
        }

    }

}

static LS_CONTROLLER: Spinlock<State> = const_spinlock(State {
    started_up: false,
    main_process: None,
    values: MemoryValues {
        loading: Pair { current: 0, old: 0 },
        level: Pair { current: 0, old: 0 },
        sublevel: Pair { current: 0, old: 0 },
        music: Pair { current: 0, old: 0 },
    },
});


#[no_mangle]
pub extern "C" fn update() {
    LS_CONTROLLER.lock().update();
}
