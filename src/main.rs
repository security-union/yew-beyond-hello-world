mod msg_ctx;

use std::rc::Rc;

use js_sys::Array;
use js_sys::Boolean;
use js_sys::JsString;
use js_sys::Reflect;
use js_sys::Uint8Array;
use msg_ctx::{MessageContext, MessageProvider};
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::*;
use yew::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct SerializableVideoChunk {
    pub chunk: Vec<u8>,
    pub timestamp: f64,
    pub duration: Option<f64>,
    pub frame_type: EncodedVideoChunkType,
}

impl Reducible for SerializableVideoChunk {
    type Action = SerializableVideoChunk;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        SerializableVideoChunk {
            chunk: action.chunk,
            timestamp: action.timestamp,
            duration: action.duration,
            frame_type: action.frame_type,
        }
        .into()
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
        chunk: vec![0, 0],
        timestamp: 0f64,
        duration: None,
        frame_type: EncodedVideoChunkType::Delta,
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
            <VideoReader/>
            <VideoRenderer/>
        </VideoChunksProvider>
    )
}

#[function_component(VideoReader)]
fn video_reader() -> Html {
    let video_context: UseReducerHandle<SerializableVideoChunk> =
        use_context::<UseReducerHandle<SerializableVideoChunk>>().unwrap();
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
                    .find(&mut |_: JsValue, _: u32, _: Array| true)
                    .unchecked_into::<VideoTrack>();

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
                        duration: video_chunk.duration(),
                        frame_type: video_chunk.type_(),
                    };
                    video_context.dispatch(data_to_transfer);
                }) as Box<dyn FnMut(JsValue)>);

                let init = VideoEncoderInit::new(
                    error_video.as_ref().unchecked_ref(),
                    output.as_ref().unchecked_ref(),
                );
                let video_encoder = VideoEncoder::new(&init).unwrap();
                let video_encoder_config = VideoEncoderConfig::new("vp8", 640u32, 480u32);
                video_encoder.configure(&video_encoder_config);

                let processor =
                    MediaStreamTrackProcessor::new(&MediaStreamTrackProcessorInit::new(
                        &video_track.unchecked_into::<MediaStreamTrack>(),
                    ))
                    .unwrap();
                let reader = processor
                    .readable()
                    .get_reader()
                    .unchecked_into::<ReadableStreamDefaultReader>();
                loop {
                    console::log_1(&JsString::from("before read"));
                    let result = JsFuture::from(reader.read()).await.map_err(|e| {
                        console::log_1(&JsString::from("error"));
                        console::log_1(&e);
                    });
                    match result {
                        Ok(js) => {
                            let video_frame = Reflect::get(&js, &JsString::from("value"))
                                .unwrap()
                                .unchecked_into::<VideoFrame>();
                            console::log_1(&JsString::from("sdfsdf"));
                            video_encoder.encode(&video_frame);
                            video_frame.close();
                        }
                        Err(e) => {
                            console::log_1(&JsString::from("result error"));
                        }
                    }
                    // console::log_1(&result);
                    console::log_1(&JsString::from("after read"));
                }
                console::log_1(&JsString::from("after calling start pulling frames"));
            });
            console::log_1(&JsString::from("closing callback"));
            || ()
        },
        (),
    );
    html!(
        <div>
            {"video reader"}
            <video autoplay={true} id="webcam"></video>
        </div>
    )
}

#[function_component(VideoRenderer)]
fn video_renderer() -> Html {
    let video_ctx = use_context::<UseReducerHandle<SerializableVideoChunk>>().unwrap();
    let video_message = video_ctx.chunk.to_owned();
    let video_decoder: UseStateHandle<Option<VideoDecoder>> = use_state(|| None);

    if (*video_decoder).is_none() {
        let error_video = Closure::wrap(Box::new(move |e: JsValue| {
            console::log_1(&JsString::from("on error"));
            console::log_1(&e);
        }) as Box<dyn FnMut(JsValue)>);

        let output = Closure::wrap(Box::new(move |chunk: JsValue| {
            console::log_1(&JsString::from("output decoded"));
            let video_chunk = chunk.unchecked_into::<HtmlImageElement>();
            let render_canvas = window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id("render")
                .unwrap()
                .unchecked_into::<HtmlCanvasElement>();

            let ctx = render_canvas
                .get_context("2d")
                .unwrap()
                .unwrap()
                .unchecked_into::<CanvasRenderingContext2d>();
            ctx.draw_image_with_html_image_element(&video_chunk, 0.0, 0.0)
                .unwrap();
        }) as Box<dyn FnMut(JsValue)>);

        let video_decoder_init = VideoDecoderInit::new(
            error_video.as_ref().unchecked_ref(),
            output.as_ref().unchecked_ref(),
        );
        error_video.forget();
        output.forget();
        let local_video_decoder = VideoDecoder::new(&video_decoder_init).unwrap();
        let video_config = VideoDecoderConfig::new("vp8");
        local_video_decoder.configure(&video_config);
        video_decoder.set(Some(local_video_decoder));
    } else if video_message.len() > 10 {
        let decoder: VideoDecoder = (*video_decoder).to_owned().unwrap();
        let data = Uint8Array::from(video_message.as_ref());
        let init = EncodedVideoChunkInit::new(&data, video_ctx.timestamp, video_ctx.frame_type);
        let encoded_video_chunk = EncodedVideoChunk::new(&init).unwrap();
        decoder.decode(&encoded_video_chunk);
    }

    html!(
        <div>
             <canvas id="render"></canvas>
        </div>
    )
}

fn main() {
    yew::start_app::<App>();
}
