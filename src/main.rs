mod msg_ctx;

use std::rc::Rc;

use js_sys::Array;
use js_sys::Boolean;
use js_sys::JsString;
use js_sys::Reflect;
use js_sys::global;
use msg_ctx::{MessageContext, MessageProvider};
use serde::Deserialize;
use serde::Serialize;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_futures::JsFuture;
use web_sys::*;
use yew::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct SerializableVideoChunk {
    pub chunk: Vec<u8>,
    pub timestamp: f64,
    pub duration: Option<f64>
}

impl Reducible for SerializableVideoChunk {
    type Action = SerializableVideoChunk;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        SerializableVideoChunk { chunk: action.chunk, timestamp: action.timestamp, duration: action.duration }.into()
    }
}

#[derive(Properties, Debug, PartialEq)]
pub struct VideoChunksProviderProps {
    #[prop_or_default]
    pub children: Children,
}

#[function_component(VideoChunksProvider)]
pub fn VideoChunksProviderImp(props: &VideoChunksProviderProps) -> Html {
    let msg = use_reducer(|| SerializableVideoChunk { 
        chunk: vec![0,0], 
        timestamp: 0f64, 
        duration: None 
    });

    html! {
        <ContextProvider<UseReducerHandle<SerializableVideoChunk>> context={msg}>
            {props.children.clone()}
        </ContextProvider<UseReducerHandle<SerializableVideoChunk>>>
    }
}


#[function_component(App)]
fn app() -> Html {
    html!(
        <VideoChunksProvider>
            <MessageProvider>
                <VideoReader/>
                <VideoRenderer/>
            </MessageProvider>
        </VideoChunksProvider>
    )
}

#[function_component(VideoReader)]
fn video_reader() -> Html {
    let video_context: UseReducerHandle<SerializableVideoChunk> = use_context::<UseReducerHandle<SerializableVideoChunk>>().unwrap();
    use_effect_with_deps(
        move |_| {
            let navigator = window().unwrap().navigator();
            let media_devices = navigator.media_devices().unwrap();
            wasm_bindgen_futures::spawn_local(async move {
                let video_element = window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .get_element_by_id("webcam")
                    .unwrap()
                    .unchecked_into::<HtmlVideoElement>();
                let mut constraints = MediaStreamConstraints::new();
                constraints.video(&Boolean::from(true));
                let devices_future = media_devices
                    .get_user_media_with_constraints(&constraints)
                    .unwrap();
                let device = JsFuture::from(devices_future)
                    .await
                    .unwrap()
                    .unchecked_into::<MediaStream>();
                console::log_1(&device);
                console::log_1(&video_element);
                video_element.set_src_object(Some(&device));
                // Get 1 video track
                let video_track = device
                    .get_video_tracks()
                    .find(&mut |_: JsValue, _: u32, _: Array| true).unchecked_into::<VideoTrack>();

                let error_video = Closure::wrap(Box::new(move |e: JsValue| {
                    console::log_1(&JsString::from("on error"));
                    console::log_1(&e);
                }) as Box<dyn FnMut(JsValue)>);

                let output = Closure::wrap(Box::new(move |chunk: JsValue| {
                    console::log_1(&JsString::from("output"));
                    let video_chunk = chunk.unchecked_into::<EncodedVideoChunk>();
                    let mut vector: Vec<u8> = vec![0; video_chunk.byte_length() as usize];
                    let chunk_data = vector.as_mut();
                    video_chunk.copy_to_with_u8_array(chunk_data);
                    let data_to_transfer = SerializableVideoChunk { 
                        chunk: Vec::from(chunk_data), 
                        timestamp: video_chunk.timestamp(), 
                        duration: video_chunk.duration() 
                    };
                    video_context.dispatch(data_to_transfer);
                    
                }) as Box<dyn FnMut(JsValue)>);

                let init = VideoEncoderInit::new(error_video.as_ref().unchecked_ref(), output.as_ref().unchecked_ref());
                let video_encoder = VideoEncoder::new(&init).unwrap();
                let video_encoder_config = VideoEncoderConfig::new("vp8", 640u32, 480u32); 
                video_encoder.configure(&video_encoder_config);

                let processor = MediaStreamTrackProcessor::new(
                    &MediaStreamTrackProcessorInit::new(&video_track.unchecked_into::<MediaStreamTrack>())
                ).unwrap();
                let reader = processor.readable().get_reader().unchecked_into::<ReadableStreamDefaultReader>();
                loop {
                    console::log_1(&JsString::from("before read"));
                    let result = JsFuture::from(reader.read()).await.map_err(|e| {
                        console::log_1(&JsString::from("error"));
                        console::log_1(&e);
                    });
                    match result {
                        Ok(js) => {
                            let video_frame = Reflect::get(&js, &JsString::from("value")).unwrap().unchecked_into::<VideoFrame>();
                            console::log_1(&JsString::from("sdfsdf"));
                            video_encoder.encode(&video_frame);
                            video_frame.close();

                        },
                        Err(e) => {
                            console::log_1(&JsString::from("result error"));
                        }
                    }
                    // console::log_1(&result);
                    console::log_1(&JsString::from("after read"));
                }
                console::log_1(&JsString::from("after calling start pulling frames"));

 
                
                // video_track.
            });
            console::log_1(&JsString::from("closing callback"));
            || ()
        },
        (),
    );

    let msg_ctx = use_context::<MessageContext>().unwrap();
    let onclick = Callback::from(move |_| msg_ctx.dispatch("Message Received.".to_string()));

    html!(
        <div>
            {"video reader"}
            <video autoplay={true} id="webcam"></video>
            <button {onclick}>
                {"PRESS ME"}
            </button>
        </div>
    )
}

#[function_component(VideoRenderer)]
fn video_renderer() -> Html {
    let msg_ctx = use_context::<MessageContext>().unwrap();
    let message = msg_ctx.inner.to_owned();
    let video_ctx = use_context::<UseReducerHandle<SerializableVideoChunk>>().unwrap();
    let video_message = video_ctx.chunk.to_owned();
    html!(
        <div>
            {"video renderer"}
             <h1>{ message }</h1>
        </div>
    )
}

fn main() {
    yew::start_app::<App>();
}
