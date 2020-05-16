use std::path::PathBuf;
use std::error::Error;
use std::sync::Arc;
use glib::clone;
use gtk::prelude::*;
use gio::prelude::*;
use gstreamer::prelude::*;

use gtk::{Application};
use gtk_resources::UIResource;

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
    video_chooser: gtk::FileChooserButton,
    output_chooser: gtk::FileChooserButton,
    execute_btn: gtk::Button,
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

fn main() {
    if std::env::var_os("GST_PLUGIN_PATH").is_none() {
        std::env::set_var("GST_PLUGIN_PATH", "./gstreamer-1.0");
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
            if let Some((logo, corner, video, output)) = gui_data(&res) {
                if let Err(err) = process(logo, corner, video, output) {
                    res.error_dialog_close_button.connect_clicked(clone!(@weak res => move |_| res.error_dialog.hide()));
                    res.error_dialog.set_property_secondary_text(Some(&format!("{}", err)));
                    res.error_dialog.show();
                    println!("{}", err);
                } else {
                    res.final_dialog_close_button.connect_clicked(clone!(@weak res => move |_| res.final_dialog.hide()));
                    res.final_dialog.show();
                }
            } else {
                res.input_error_dialog_close_button.connect_clicked(clone!(@weak res => move |_| res.input_error_dialog.hide()));
                res.input_error_dialog.show();
            }
        }));
        res.window.show_all();
    }));

    application.run(&[]);
}

fn gui_data(res: &AppResource) -> Option<(PathBuf, Corner, PathBuf, PathBuf)> {
    let logo = res.logo_chooser.get_filename()?;
    let video = res.video_chooser.get_filename()?;
    let output = res.output_chooser.get_filename()?;
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
    Some((logo, corner, video, output))
}

fn process(logo_path: PathBuf, corner: Corner, video_path: PathBuf, output_dir: PathBuf) -> Result<(), Box<dyn Error>> {
    let pipeline = gstreamer::parse_launch(&PIPELINE_STR)?
        .downcast::<gstreamer::Pipeline>()
        .unwrap();

    let input = pipeline.get_by_name("input").unwrap();
    input.set_property("location", &video_path.to_str().unwrap())?;

    let logo = pipeline.get_by_name("logo").unwrap();
    logo.set_property("location", &logo_path.to_str().unwrap())?;
    match corner {
        Corner::TopLeft => { logo.set_property("offset-x", &20)?; logo.set_property("offset-y", &20)?; }
        Corner::BottomLeft => { logo.set_property("offset-x", &20)?; logo.set_property("offset-y", &-20)?; }
        Corner::TopRight => { logo.set_property("offset-x", &-20)?; logo.set_property("offset-y", &20)?; }
        Corner::BottomRight => { logo.set_property("offset-x", &-20)?; logo.set_property("offset-y", &-20)?; }
    }

    let output = pipeline.get_by_name("output").unwrap();
    output.set_property("location", &format!("{}/video-com-logo.mp4", output_dir.to_str().unwrap()))?;

    let bus = pipeline.get_bus().unwrap();

    pipeline.set_state(gstreamer::State::Playing)?;

    for msg in bus.iter_timed(gstreamer::CLOCK_TIME_NONE) {
        match msg.view() {
            gstreamer::MessageView::Eos(..) => break,
            gstreamer::MessageView::Error(err) => return Err(Box::new(err.get_error())),
            _ => (),
        }
    }

    pipeline.set_state(gstreamer::State::Null)?;
    Ok(())
}
