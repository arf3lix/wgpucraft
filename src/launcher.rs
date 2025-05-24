use winit:: {
    event::{Event, StartCause},
    event_loop::ControlFlow,
};
use log::{trace, debug, info, warn, error};
use tracy::zone;

use winit::{
    event_loop::EventLoop,
        window::WindowBuilder,
    };

use crate::State;

pub fn run() {
    
    env_logger::init();

    info!("This is an info message");


    let event_loop = EventLoop::new().unwrap();


    event_loop.set_control_flow(ControlFlow::Poll);

    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let mut state = State::new(&window);
    state.initialize();
    
    event_loop.run(move | event, elwt: &winit::event_loop::EventLoopWindowTarget<()> | {

        match event {
            Event::WindowEvent {
                window_id,
                event
            }
            if window_id == state.window.id() => {

                state.handle_window_event(event, elwt)
            }
            Event::DeviceEvent { ref event, .. } => {
                zone!("handling device input"); // <- Marca el inicio del bloque

                state.handle_device_input(event, elwt);
            }
            Event::AboutToWait => {
                state.handle_wait(elwt);
            }
            _ => ()
        }
    }).unwrap();
}