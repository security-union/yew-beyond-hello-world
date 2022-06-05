use js_sys::Array;
use js_sys::Boolean;
use js_sys::JsString;
use js_sys::Reflect;
use js_sys::Uint8Array;
use std::rc::Rc;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::*;
use yew::prelude::*;

static VIDEO_CODEC: &str = "av01.0.01M.08";

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
        chunk: vec![],
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
            <div class="grid">
                <VideoProducer/>
                <VideoConsumer/>
            </div>
        </VideoChunksProvider>
    )
}

#[function_component(VideoProducer)]
fn video_producer() -> Html {
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
                video_element.set_src_object(Some(&device));
                let video_track = Box::new(
                    device
                        .get_video_tracks()
                        .find(&mut |_: JsValue, _: u32, _: Array| true)
                        .unchecked_into::<VideoTrack>(),
                );

                let error_video = Closure::wrap(Box::new(move |e: JsValue| {
                    console::log_1(&JsString::from("on error"));
                    console::log_1(&e);
                }) as Box<dyn FnMut(JsValue)>);

                let output = Closure::wrap(Box::new(move |chunk: JsValue| {
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
                let settings = &video_track
                    .clone()
                    .unchecked_into::<MediaStreamTrack>()
                    .get_settings();
                let width = Reflect::get(&settings, &JsString::from("width"))
                    .unwrap()
                    .as_f64()
                    .unwrap();
                let height = Reflect::get(&settings, &JsString::from("height"))
                    .unwrap()
                    .as_f64()
                    .unwrap();
                let video_encoder_config =
                    VideoEncoderConfig::new(&VIDEO_CODEC, height as u32, width as u32);
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
                    let result = JsFuture::from(reader.read()).await.map_err(|e| {
                        console::log_1(&e);
                    });
                    match result {
                        Ok(js) => {
                            let video_frame = Reflect::get(&js, &JsString::from("value"))
                                .unwrap()
                                .unchecked_into::<VideoFrame>();
                            video_encoder.encode(&video_frame);
                            video_frame.close();
                        }
                        Err(_e) => {
                            console::log_1(&JsString::from("Unable to get frame from camera"));
                        }
                    }
                }
            });
            || ()
        },
        (),
    );
    html!(
        <div id="video-producer">
            <h3>{"Producer"}</h3>
            <video autoplay={true} id="webcam"></video>
        </div>
    )
}

#[function_component(VideoConsumer)]
fn video_consumer() -> Html {
    let video_ctx = use_context::<UseReducerHandle<SerializableVideoChunk>>().unwrap();
    let video_message = video_ctx.chunk.to_owned();
    let video_decoder: UseStateHandle<Option<VideoDecoder>> = use_state(|| None);

    if (*video_decoder).is_none() {
        let error_video = Closure::wrap(Box::new(move |e: JsValue| {
            console::log_1(&e);
        }) as Box<dyn FnMut(JsValue)>);

        let output = Closure::wrap(Box::new(move |original_chunk: JsValue| {
            let chunk = Box::new(original_chunk);
            let video_chunk = chunk.clone().unchecked_into::<HtmlImageElement>();
            let width = Reflect::get(&chunk.clone(), &JsString::from("codedWidth"))
                .unwrap()
                .as_f64()
                .unwrap();
            let height = Reflect::get(&chunk.clone(), &JsString::from("codedHeight"))
                .unwrap()
                .as_f64()
                .unwrap();
            let height = Reflect::get(&chunk.clone(), &JsString::from("codedHeight"))
                .unwrap()
                .as_f64()
                .unwrap();
            let render_canvas = window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id("render")
                .unwrap()
                .unchecked_into::<HtmlCanvasElement>();
            render_canvas.set_width(width as u32);
            render_canvas.set_height(height as u32);
            let ctx = render_canvas
                .get_context("2d")
                .unwrap()
                .unwrap()
                .unchecked_into::<CanvasRenderingContext2d>();
            ctx.draw_image_with_html_image_element(&video_chunk, 0.0, 0.0)
                .unwrap();
            video_chunk.unchecked_into::<VideoFrame>().close();
        }) as Box<dyn FnMut(JsValue)>);

        let video_decoder_init = VideoDecoderInit::new(
            error_video.as_ref().unchecked_ref(),
            output.as_ref().unchecked_ref(),
        );
        error_video.forget();
        output.forget();
        let local_video_decoder = VideoDecoder::new(&video_decoder_init).unwrap();
        let video_config = VideoDecoderConfig::new(&VIDEO_CODEC);
        local_video_decoder.configure(&video_config);
        video_decoder.set(Some(local_video_decoder));
    } else if !video_message.is_empty() {
        let decoder: VideoDecoder = (*video_decoder).to_owned().unwrap();
        let data = Uint8Array::from(video_message.as_ref());
        let init = EncodedVideoChunkInit::new(&data, video_ctx.timestamp, video_ctx.frame_type);
        let encoded_video_chunk = EncodedVideoChunk::new(&init).unwrap();
        decoder.decode(&encoded_video_chunk);
    }

    html!(
        <div id="video-consumer">
            <h3>{"Consumer"}</h3>
             <canvas id="render"></canvas>
        </div>
    )
}

fn main() {
    yew::start_app::<App>();
}
