#![windows_subsystem = "windows"]

use std::env;
use std::path::PathBuf;
use std::error::Error;
use std::sync::Arc;
use glib::clone;
use gtk::prelude::*;
use gio::prelude::*;
use gstreamer::prelude::*;
use futures::prelude::*;

use gtk::Application;
use gtk_resources::UIResource;
use image::GenericImageView;

static PIPELINE_STR: &str = "filesrc name=input ! decodebin name=dec ! gdkpixbufoverlay name=logo ! videoscale ! x264enc ! queue ! mp4mux name=mux ! filesink name=output dec. ! audioconvert ! lamemp3enc ! queue ! mux.";

#[derive(UIResource)]
#[resource = "/com/paulolieuthier/logoembedder/app.ui"]
struct AppResource {
    window: gtk::ApplicationWindow,
    logo_chooser: gtk::FileChooserButton,
    logo_topleft_radio: gtk::RadioButton,
    logo_bottomleft_radio: gtk::RadioButton,
    logo_topright_radio: gtk::RadioButton,
    logo_bottomright_radio: gtk::RadioButton,
    logo_width_scale: gtk::Scale,
    video_chooser: gtk::FileChooserButton,
    execute_btn: gtk::Button,
    loading_spinner: gtk::Spinner,
    input_error_dialog: gtk::Dialog,
    input_error_dialog_close_button: gtk::Button,
    error_dialog: gtk::MessageDialog,
    error_dialog_close_button: gtk::Button,
    final_dialog: gtk::Dialog,
    final_dialog_close_button: gtk::Button,
}

enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

struct Parameters {
    logo_path: PathBuf,
    logo_position: Corner,
    logo_width: f64,
    video_path: PathBuf,
}

fn main() -> std::io::Result<()> {
    if std::env::var_os("GST_PLUGIN_PATH").is_none() {
        let mut plugins_path = std::env::current_exe()?;
        plugins_path.pop();
        std::env::set_var("GST_PLUGIN_PATH", &format!("{}", plugins_path.display()));
    }

    gstreamer::init().unwrap();
    gtk::init().unwrap();

    // ui resources
    let res_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/app.gresource"));
    let data = glib::Bytes::from(&res_bytes[..]);
    let resource = gio::Resource::new_from_data(&data).unwrap();
    gio::resources_register(&resource);

    let application = Application::new(Some("com.paulolieuthier.logoembedder"), Default::default())
        .expect("failed to initialize GTK application");
    let res = Arc::new(AppResource::load().unwrap());

    application.connect_activate(clone!(@weak res => move |app| {
        res.window.set_application(Some(app));
        res.execute_btn.connect_clicked(clone!(@weak res => move |_| {
            if let Some(params) = gui_data(&res) {
                res.execute_btn.set_sensitive(false);
                res.loading_spinner.set_property_active(true);

                let result = glib::MainContext::default().block_on(process(params));

                if let Err(err) = result {
                    res.error_dialog_close_button.connect_clicked(clone!(@weak res => move |_| res.error_dialog.hide()));
                    res.error_dialog.set_property_secondary_text(Some(&format!("{}", err)));
                    res.error_dialog.show();
                    println!("{}", err);
                } else {
                    res.final_dialog_close_button.connect_clicked(clone!(@weak res => move |_| res.final_dialog.hide()));
                    res.final_dialog.show();
                }

                res.execute_btn.set_sensitive(true);
                res.loading_spinner.set_property_active(false);
            } else {
                res.input_error_dialog_close_button.connect_clicked(clone!(@weak res => move |_| res.input_error_dialog.hide()));
                res.input_error_dialog.show();
            }
        }));
        res.window.show_all();
    }));

    application.run(&[]);
    Ok(())
}

fn gui_data(res: &AppResource) -> Option<Parameters> {
    let logo = res.logo_chooser.get_filename()?;
    let video = res.video_chooser.get_filename()?;
    let corner =
        if res.logo_bottomright_radio.get_active() {
            Corner::BottomRight
        } else if res.logo_bottomleft_radio.get_active() {
            Corner::BottomLeft
        } else if res.logo_topright_radio.get_active() {
            Corner::TopRight
        } else if res.logo_topleft_radio.get_active() {
            Corner::TopLeft
        } else {
            unreachable!();
        };
    let width = res.logo_width_scale.get_value();
    Some(Parameters { logo_path: logo, logo_position: corner, logo_width: width, video_path: video })
}

async fn process(params: Parameters) -> Result<(), Box<dyn Error>> {
    let pipeline = gstreamer::parse_launch(&PIPELINE_STR)?
        .downcast::<gstreamer::Pipeline>()
        .unwrap();

    let Parameters { logo_path, logo_position, logo_width, video_path } = params;

    let input = pipeline.get_by_name("input").unwrap();
    input.set_property("location", &video_path.to_str().unwrap())?;

    let logo = pipeline.get_by_name("logo").unwrap();

    match logo_position {
        Corner::TopLeft => { logo.set_property("offset-x", &20)?; logo.set_property("offset-y", &20)?; }
        Corner::BottomLeft => { logo.set_property("offset-x", &20)?; logo.set_property("offset-y", &-20)?; }
        Corner::TopRight => { logo.set_property("offset-x", &-20)?; logo.set_property("offset-y", &20)?; }
        Corner::BottomRight => { logo.set_property("offset-x", &-20)?; logo.set_property("offset-y", &-20)?; }
    }

    // logo size calculation
    let logo_img = image::open(&logo_path)?;
    let (img_width, img_height) = logo_img.dimensions();
    let (img_width, img_height) = (img_width as f64, img_height as f64);
    let logo_height = img_height * logo_width / img_width;
    let (logo_width, logo_height) = (logo_width as i32, logo_height as i32);

    // resizing
    let mut resized_img = env::temp_dir();
    resized_img.push(&format!("cfclogo-{}.png", (rand::random::<f32>() * 1000 as f32) as u32));
    image::imageops::resize(&logo_img, logo_width as u32, logo_height as u32, image::imageops::FilterType::CatmullRom).save(&resized_img)?;

    logo.set_property("overlay-width", &logo_width)?;
    logo.set_property("overlay-height", &logo_height)?;
    logo.set_property("location", &resized_img.to_str().unwrap())?;

    let original_name = video_path.clone().with_extension("");
    let new_file_name = format!("{}-com-logo", original_name.file_name().unwrap().to_str().unwrap());
    let mut new_file = original_name.clone();
    new_file.pop();
    new_file.push(new_file_name);
    new_file.set_extension("mp4");
    let output = pipeline.get_by_name("output").unwrap();
    output.set_property("location", &format!("{}", new_file.display()))?;

    pipeline.set_state(gstreamer::State::Playing)?;

    let mut messages = pipeline.get_bus().unwrap().stream();
    while let Some(msg) = messages.next().await {
        match msg.view() {
            gstreamer::MessageView::Eos(..) => break,
            gstreamer::MessageView::Error(err) => return Err(Box::new(err.get_error())),
            _ => (),
        };
    }

    pipeline.set_state(gstreamer::State::Null)?;
    Ok(())
}
