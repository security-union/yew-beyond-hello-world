use std::rc::Rc;

use wasm_bindgen_futures::JsFuture;
use web_sys::*;
use yew::prelude::*;
use wasm_bindgen::{JsCast, prelude::Closure, JsValue};
use js_sys::*;

static VIDEO_CODEC: &str = "vp09.00.10.08";
const VIDEO_HEIGHT: i32 = 720i32;
const VIDEO_WIDTH: i32 = 1280i32;

#[derive(Clone, Debug, PartialEq)]
struct EncodedVideoChunkWrapper {
    pub chunk: Option<EncodedVideoChunk>
}

impl Reducible for EncodedVideoChunkWrapper {
    type Action = EncodedVideoChunkWrapper;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        action.clone().into()
    }
}

#[derive(Properties, Debug, PartialEq)]
pub struct VideoChunksProviderProps {
    #[prop_or_default]
    pub children: Children
}

#[function_component(VideoChunksProvider)]
pub fn VideoChunksProviderImp(props: &VideoChunksProviderProps) -> Html {
    let msg = use_reducer( || EncodedVideoChunkWrapper {
        chunk: None
    });

    html! {
        <ContextProvider<UseReducerHandle<EncodedVideoChunkWrapper>> context={msg}>
            {props.children.clone()}
        </ContextProvider<UseReducerHandle<EncodedVideoChunkWrapper>>>
    }
}

#[function_component(Producer)]
fn producer() -> Html {
    let video_context = use_context::<UseReducerHandle<EncodedVideoChunkWrapper>>().unwrap();
    use_effect_with_deps( move |_| {
        wasm_bindgen_futures::spawn_local( async move {
            let navigator = window().unwrap().navigator();
            let media_devices = navigator.media_devices().unwrap();
            let video_element = window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id("webcam")
                .unwrap()
                .unchecked_into::<HtmlVideoElement>();
            
            let mut constraints = MediaStreamConstraints::new();
            constraints.video(&Boolean::from(true));
            let devices_query = media_devices
            .get_user_media_with_constraints(&constraints).unwrap();
            let device = JsFuture::from(devices_query)
                .await
                .unwrap()
                .unchecked_into::<MediaStream>();
            video_element.set_src_object(Some(&device));
            let video_track = Box::new(
                device.get_video_tracks()
                .find(&mut |_: JsValue, _:u32, _:Array | true)
                .unchecked_into::<VideoTrack>()
            );

            let error_handler = Closure::wrap(Box::new(move |e:JsValue| {
                console::log_1(&JsString::from("on errror"));
                console::log_1(&e);
            }) as Box<dyn FnMut(JsValue)>);

            let output_handler = Closure::wrap(Box::new(move | chunk: JsValue| {
                let video_chunk = chunk.unchecked_into::<EncodedVideoChunk>();
                video_context.dispatch(
                    EncodedVideoChunkWrapper { chunk: Some(video_chunk)}
                );
            }) as Box<dyn FnMut(JsValue)>);
            let video_encoder_init = VideoEncoderInit::new(
                error_handler.as_ref().unchecked_ref(),
                output_handler.as_ref().unchecked_ref()
            );
            let video_encoder = VideoEncoder::new(&video_encoder_init).unwrap();
            let settings = &mut video_track
                .clone()
                .unchecked_into::<MediaStreamTrack>()
                .get_settings();
            settings.width(VIDEO_WIDTH);
            settings.height(VIDEO_HEIGHT);
            let video_encoder_config = VideoEncoderConfig::new(
                &VIDEO_CODEC,
                VIDEO_HEIGHT as u32,
                VIDEO_WIDTH as u32
            );
            video_encoder.configure(&video_encoder_config);
            let processor = MediaStreamTrackProcessor::new(
                &MediaStreamTrackProcessorInit::new(
                    &video_track.unchecked_into::<MediaStreamTrack>(),
                )
            ).unwrap();
            let reader = processor
                .readable()
                .get_reader()
                .unchecked_into::<ReadableStreamDefaultReader>();
            loop {
                let result = JsFuture::from(reader.read())
                    .await
                    .map_err(|e| {
                        console::log_1(&e);
                    });
                match result {
                    Ok(js_frame) => {
                        let video_frame = 
                        Reflect::get(&js_frame, &JsString::from("value"))
                        .unwrap()
                        .unchecked_into::<VideoFrame>();
                        video_encoder.encode(&video_frame);
                        video_frame.close();
                    }
                    Err(_e) => {
                        console::log_1(&JsString::from("error"));
                    }
                }
            }
        });
    || ()
    }, (),
    );
        
    html!(
        <div class="producer">
            <h3>{"Producer"}</h3>
            <video autoplay=true id="webcam"></video>
        </div>
    )
}

#[function_component(Consumer)]
fn consumer() -> Html {
    let video_ctx = use_context::<UseReducerHandle<EncodedVideoChunkWrapper>>().unwrap();
    let video_decoder: UseStateHandle<Option<VideoDecoder>> = use_state(|| None);
    if (*video_decoder).is_none() {
        let error_video = Closure::wrap(Box::new(move |e: JsValue| {
            console::log_1(&e);
        }) as Box <dyn FnMut(JsValue)>);

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
            ctx.draw_image_with_html_image_element(
                &video_chunk,
                0.0,
                0.0
            );
            video_chunk.unchecked_into::<VideoFrame>().close();
        }) as Box <dyn FnMut(JsValue)>);

        let local_video_decoder = VideoDecoder::new(
            &VideoDecoderInit::new(
                error_video.as_ref().unchecked_ref(),
                output.as_ref().unchecked_ref()
            )
        ).unwrap();
        error_video.forget();
        output.forget();
        local_video_decoder.configure(&VideoDecoderConfig::new(&VIDEO_CODEC));
        video_decoder.set(Some(local_video_decoder));
    } else if !(*video_ctx).chunk.is_none() {
        let chunk = (*video_ctx).chunk.as_ref().unwrap();
        let mut video_vector = vec![0u8; chunk.byte_length() as usize];
        let video_message = video_vector.as_mut();
        chunk.copy_to_with_u8_array(video_message);
        let decoder: VideoDecoder =(*video_decoder).to_owned().unwrap();
        let data = Uint8Array::from(video_message.as_ref());
        let encoded_video_chunk = EncodedVideoChunk::new(
            &EncodedVideoChunkInit::new(&data, chunk.timestamp(), chunk.type_())
        ).unwrap();
        decoder.decode(&encoded_video_chunk);
    }
    html!(
        <div class="consumer">
            <h3>{"Consumer"}</h3>
            <canvas id="render"></canvas>
        </div>
    )
}

#[function_component(App)]
fn app() -> Html {
    html!(
        <VideoChunksProvider>
            <div class="grid">
                <Producer/>
                <Consumer/>
            </div>
        </VideoChunksProvider>
    )
}

fn main() {
    yew::start_app::<App>();
}
