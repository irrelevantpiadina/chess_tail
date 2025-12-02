use macroquad::{prelude::*, ui};

pub fn standard(font: &Font) -> ui::Skin {
    let really_corporate_looking_gray = Color::new(0.2, 0.2, 0.2, 1.0);

    let window_style = ui::root_ui()
        .style_builder()
        .color(really_corporate_looking_gray)
        .text_color(WHITE)
        .with_font(font)
        .unwrap()
        .build();

    let button_style = ui::root_ui()
        .style_builder()
        .color(GRAY)
        .text_color(WHITE)
        .margin(RectOffset {
            left: 5.0,
            right: 5.0,
            bottom: 5.0,
            top: 5.0,
        })
        .color_hovered(LIGHTGRAY)
        .with_font(font)
        .unwrap()
        .build();

    let tabbar_style = ui::root_ui()
        .style_builder()
        .color(GRAY)
        .text_color(WHITE)
        .color_hovered(LIGHTGRAY)
        .color_selected(GRAY)
        .color_selected_hovered(GRAY)
        .font_size(13)
        .with_font(font)
        .unwrap()
        .build();

    let combobox_style = ui::root_ui()
        .style_builder()
        .color(DARKGRAY)
        .color_hovered(GRAY)
        .text_color(WHITE)
        .margin(RectOffset {
            left: 10.0,
            right: 10.0,
            bottom: 10.0,
            top: 10.0,
        })
        .font_size(13)
        .with_font(font)
        .unwrap()
        .build();

    let label_style = ui::root_ui()
        .style_builder()
        .text_color(WHITE)
        .font_size(15)
        .with_font(font)
        .unwrap()
        .build();

    let window_titlebar_style = ui::root_ui()
        .style_builder()
        .text_color(WHITE)
        .with_font(font)
        .unwrap()
        .build();

    let editbox_style = ui::root_ui()
        .style_builder()
        .text_color(WHITE)
        .color(DARKGRAY)
        .color_selected(GRAY)
        .color_selected_hovered(DARKGRAY)
        .color_hovered(DARKGRAY)
        .color_clicked(DARKGRAY)
        .margin(RectOffset {
            left: 5.0,
            right: 5.0,
            bottom: 5.0,
            top: 5.0,
        })
        .with_font(font)
        .unwrap()
        .build();

    let scrollbar_handle_style = ui::root_ui()
        .style_builder()
        .color(really_corporate_looking_gray)
        .build();

    ui::Skin {
        window_style,
        button_style,
        tabbar_style,
        combobox_style,
        label_style,
        window_titlebar_style,
        editbox_style,
        scrollbar_handle_style,
        title_height: 30.0,
        ..ui::root_ui().default_skin()
    }
}
