use winit:: {
    event::Event,
    event_loop::ControlFlow,
};
//use log::{trace, debug, info, warn, error};

use winit::{
    event_loop::EventLoop,
        window::WindowBuilder,
    };

use crate::Engine;

pub async fn run() {

    env_logger::init();

    // debug!("This is a debug message");
    //info!("This is an info message");
    // warn!("This is a warning message");
    // error!("This is an error message");

    //TODO: establish this parameters from settings
    let runtime = tokio::runtime::Builder::new_current_thread()
        .worker_threads(4)
        .thread_name("wgpucraft")
        .thread_stack_size(3 * 1024 * 1024)
        .build()
        .unwrap();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let mut engine = Engine::new(&window, runtime).await;
    engine.initialize();
    
    event_loop.run(move | event, elwt: &winit::event_loop::EventLoopWindowTarget<()> | {
        match event {
            Event::WindowEvent {
                window_id,
                event
            }
            if window_id == engine.window.id() => {
                engine.handle_window_event(event, elwt)
            }
            Event::DeviceEvent { ref event, .. } => {
                engine.handle_device_input(event, elwt);
            }
            Event::AboutToWait => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                engine.window.request_redraw();
            }
            _ => ()
        }
    }).unwrap();
}