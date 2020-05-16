#!/usr/bin/env sh
export PKG_CONFIG_ALLOW_CROSS=1
export PKG_CONFIG_PATH=/usr/x86_64-w64-mingw32/sys-root/mingw/lib/pkgconfig
cargo build --target=x86_64-pc-windows-gnu --release

GTK_INSTALL_PATH=/usr/x86_64-w64-mingw32/sys-root/mingw
rm -rf windows
mkdir windows
cp target/x86_64-pc-windows-gnu/release/*.exe windows
cp $GTK_INSTALL_PATH/bin/*.dll windows
mkdir -p windows/gstreamer-1.0
cp $GTK_INSTALL_PATH/lib/gstreamer-1.0/*.dll windows/gstreamer-1.0
mkdir -p windows/share/glib-2.0/schemas
mkdir -p windows/share/icons
cp -r $GTK_INSTALL_PATH/share/glib-2.0/schemas/* windows/share/glib-2.0/schemas
cp -r $GTK_INSTALL_PATH/share/icons/* windows/share/icons
mkdir -p windows/share/{gtk-3.0,themes}
cp -r res/themes/Windows10 windows/share/themes/Windows10
cp res/themes/gtk-settings.ini windows/share/gtk-3.0/settings.ini
