#[macro_export]
macro_rules! emit {
    (Quit($opt:expr)) => {
        yazi_shared::event::Event::Quit($opt).emit();
    };
    (Call($cmd:expr)) => {
        yazi_shared::event::Event::Call($cmd).emit();
    };
    (Seq($cmds:expr)) => {
        yazi_shared::event::Event::Seq($cmds).emit();
    };
    ($event:ident) => {
        yazi_shared::event::Event::$event.emit();
    };
}

#[macro_export]
macro_rules! relay {
    // 先返回一个简单的 Cmd：形如 "ui:open"
    ($layer:ident : $name:ident) => {
        yazi_shared::event::Cmd::new(concat!(stringify!($layer), ":", stringify!($name)))
    };
    // 预留带参数版本，现在先忽略参数（后续把 Cmd 扩展出 args 再接入）
    ($layer:ident : $name:ident, $args:expr) => {
        yazi_shared::event::Cmd::new(concat!(stringify!($layer), ":", stringify!($name)))
    };
}
