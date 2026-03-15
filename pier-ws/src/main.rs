use std::io;

pier_macro::mod_pub!(app filetree);
// pier_macro::mod_flat!(app filetree);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  // pier_ui::ui::draw_ui()?;
    app::App::serve().await
}
