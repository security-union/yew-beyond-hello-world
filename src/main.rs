use std::result;

use web_sys::{window, console, MediaStreamConstraints};
use yew::prelude::*;
use wasm_bindgen_futures::JsFuture;
use js_sys::{Array, Object, JsString, Boolean};
use wasm_bindgen::JsValue;

#[function_component(App)]
fn app() -> Html {
    html!(
        <div>
            <VideoReader/>
            <VideoRenderer/>
        </div>
    )
}

#[function_component(VideoReader)]
fn video_reader() -> Html {
    html!(
        <div>
            {"video reader"}
        </div>
    )
}

#[function_component(VideoRenderer)]
fn video_renderer() -> Html {
    use_effect_with_deps(move |_| {
        let navigator = window().unwrap().navigator();
        let media_devices = navigator.media_devices().unwrap();
        
        wasm_bindgen_futures::spawn_local(async move {
            let video_element = web_sys::window().unwrap().document().unwrap().get_element_by_id("webcam").unwrap();
            let property = JsString::from("srcObject");
            let mut constraints = MediaStreamConstraints::new();
            constraints.video(&Boolean::from(true));
            let devices2 = media_devices.get_user_media_with_constraints(&constraints).unwrap();
            let device = JsFuture::from(devices2).await.unwrap();
            console::log_1(&device);
            console::log_1(&video_element);
            let set_result = js_sys::Reflect::set(&video_element, &property, &device).map_err(|e| {
                console::error_1(&e);
            }).unwrap();
            console::log_1(&Boolean::from(set_result));
        });
        || ()
    }, ());

    html!(
        <div>
            {"video renderer"}
            <video autoplay={true} id="webcam"></video>
        </div>
    )
}


fn main() {
    yew::start_app::<App>();
}