#[macro_export(local_inner_macros)]
macro_rules! o(
    ($($args:tt)*) => {
        slog::OwnedKV($crate::kv!($($args)*))
    };
);

#[macro_export(local_inner_macros)]
macro_rules! b(
    ($($args:tt)*) => {
        slog::BorrowedKV(&kv!($($args)*))
    };
);

#[macro_export(local_inner_macros)]
macro_rules! kv(
    (@ $args_ready:expr; $k:ident = %$v:expr) => {
        $crate::kv!(@ (slog::SingleKV::from((slog::__slog_builtin(@stringify $k), slog::__slog_builtin!(@format_args "{}", $v))), $args_ready); )
    };
    (@ $args_ready:expr; $k:ident = %$v:expr, $($args:tt)* ) => {
        $crate::kv!(@ (slog::SingleKV::from((slog::__slog_builtin!(@stringify $k), slog::__slog_builtin!(@format_args "{}", $v))), $args_ready); $($args)* )
    };
    (@ $args_ready:expr; $k:ident = #%$v:expr) => {
        $crate::kv!(@ (slog::SingleKV::from((slog::__slog_builtin!(@stringify $k), slog::__slog_builtin!(@format_args "{:#}", $v))), $args_ready); )
    };
    (@ $args_ready:expr; $k:ident = #%$v:expr, $($args:tt)* ) => {
        $crate::kv!(@ (slog::SingleKV::from((slog::__slog_builtin!(@stringify $k), slog::__slog_builtin!(@format_args "{:#}", $v))), $args_ready); $($args)* )
    };
    (@ $args_ready:expr; $k:ident = ?$v:expr) => {
        $crate::kv!(@ (slog::SingleKV::from((slog::__slog_builtin!(@stringify $k), slog::__slog_builtin!(@format_args "{:?}", $v))), $args_ready); )
    };
    (@ $args_ready:expr; $k:ident = ?$v:expr, $($args:tt)* ) => {
        $crate::kv!(@ (slog::SingleKV::from((slog::__slog_builtin!(@stringify $k), slog::__slog_builtin!(@format_args "{:?}", $v))), $args_ready); $($args)* )
    };
    (@ $args_ready:expr; $k:ident = #?$v:expr) => {
        $crate::kv!(@ (slog::SingleKV::from((slog::__slog_builtin!(@stringify $k), slog::__slog_builtin!(@format_args "{:#?}", $v))), $args_ready); )
    };
    (@ $args_ready:expr; $k:ident = #?$v:expr, $($args:tt)* ) => {
        $crate::kv!(@ (slog::SingleKV::from((slog::__slog_builtin!(@stringify $k), slog::__slog_builtin!(@format_args "{:#?}", $v))), $args_ready); $($args)* )
    };
    (@ $args_ready:expr; $k:ident = #$v:expr) => {
        $crate::kv!(@ (slog::SingleKV::from((slog::__slog_builtin!(@stringify $k), slog::ErrorValue($v))), $args_ready); )
    };
    (@ $args_ready:expr; $k:ident = $v:expr) => {
        $crate::kv!(@ (slog::SingleKV::from((slog::__slog_builtin!(@stringify $k), $v)), $args_ready); )
    };
    (@ $args_ready:expr; $k:ident = $v:expr, $($args:tt)* ) => {
        $crate::kv!(@ (slog::SingleKV::from((slog::__slog_builtin!(@stringify $k), $v)), $args_ready); $($args)* )
    };
    (@ $args_ready:expr; $kv:expr) => {
        $crate::kv!(@ ($kv, $args_ready); )
    };
    (@ $args_ready:expr; $kv:expr, $($args:tt)* ) => {
        $crate::kv!(@ ($kv, $args_ready); $($args)* )
    };
    (@ $args_ready:expr; ) => {
        $args_ready
    };
    (@ $args_ready:expr;, ) => {
        $args_ready
    };
    ($($args:tt)*) => {
        $crate::kv!(@ (); $($args)*)
    };
);

// Verbose, filled with details, not normally required
#[macro_export(local_inner_macros)]
macro_rules! trace(
    ($l:expr, $($args:tt)*) => {
        log!($l, slog::Level::Trace, "", $($args)*)
    };
);

// Decisions
#[macro_export(local_inner_macros)]
macro_rules! info(
    ($l:expr, $($args:tt)*) => {
        log!($l, slog::Level::Info, "", $($args)*)
    };
);

// Invalid behavior, but not critical
#[macro_export(local_inner_macros)]
macro_rules! warn(
    ($l:expr, $($args:tt)*) => {
        log!($l, slog::Level::Warning, "", $($args)*)
    };
);

// Critical failures
#[macro_export(local_inner_macros)]
macro_rules! err(
    ($l:expr, $($args:tt)*) => {
        log!($l, slog::Level::Error, "", $($args)*)
    };
);

#[macro_export(local_inner_macros)]
macro_rules! log(
    // `2` means that `;` was already found
   (2 @ { $($fmt:tt)* }, { $($kv:tt)* },  $l:expr, $lvl:expr, $tag:expr, $msg_fmt:expr) => {
      slog::Logger::log(&$l, &slog::record!($lvl, $tag, &slog::__slog_builtin!(@format_args $msg_fmt, $($fmt)*), $crate::b!($($kv)*)))
   };
   (2 @ { $($fmt:tt)* }, { $($kv:tt)* }, $l:expr, $lvl:expr, $tag:expr, $msg_fmt:expr,) => {
       log!(2 @ { $($fmt)* }, { $($kv)* }, $l, $lvl, $tag, $msg_fmt)
   };
   (2 @ { $($fmt:tt)* }, { $($kv:tt)* }, $l:expr, $lvl:expr, $tag:expr, $msg_fmt:expr;) => {
       log!(2 @ { $($fmt)* }, { $($kv)* }, $l, $lvl, $tag, $msg_fmt)
   };
   (2 @ { $($fmt:tt)* }, { $($kv:tt)* }, $l:expr, $lvl:expr, $tag:expr, $msg_fmt:expr, $($args:tt)*) => {
       log!(2 @ { $($fmt)* }, { $($kv)* $($args)*}, $l, $lvl, $tag, $msg_fmt)
   };
    // `1` means that we are still looking for `;`
    // -- handle named arguments to format string
   (1 @ { $($fmt:tt)* }, { $($kv:tt)* }, $l:expr, $lvl:expr, $tag:expr, $msg_fmt:expr, $k:ident = $v:expr) => {
       log!(2 @ { $($fmt)* $k = $v }, { $($kv)* slog::__slog_builtin!(@stringify $k) => $v, }, $l, $lvl, $tag, $msg_fmt)
   };
   (1 @ { $($fmt:tt)* }, { $($kv:tt)* }, $l:expr, $lvl:expr, $tag:expr, $msg_fmt:expr, $k:ident = $v:expr;) => {
       log!(2 @ { $($fmt)* $k = $v }, { $($kv)* slog::__slog_builtin!(@stringify $k) => $v, }, $l, $lvl, $tag, $msg_fmt)
   };
   (1 @ { $($fmt:tt)* }, { $($kv:tt)* }, $l:expr, $lvl:expr, $tag:expr, $msg_fmt:expr, $k:ident = $v:expr,) => {
       log!(2 @ { $($fmt)* $k = $v }, { $($kv)* slog::__slog_builtin!(@stringify $k) => $v, }, $l, $lvl, $tag, $msg_fmt)
   };
   (1 @ { $($fmt:tt)* }, { $($kv:tt)* }, $l:expr, $lvl:expr, $tag:expr, $msg_fmt:expr, $k:ident = $v:expr; $($args:tt)*) => {
       log!(2 @ { $($fmt)* $k = $v }, { $($kv)* slog::__slog_builtin!(@stringify $k) => $v, }, $l, $lvl, $tag, $msg_fmt, $($args)*)
   };
   (1 @ { $($fmt:tt)* }, { $($kv:tt)* }, $l:expr, $lvl:expr, $tag:expr, $msg_fmt:expr, $k:ident = $v:expr, $($args:tt)*) => {
       log!(1 @ { $($fmt)* $k = $v, }, { $($kv)* slog::__slog_builtin!(@stringify $k) => $v, }, $l, $lvl, $tag, $msg_fmt, $($args)*)
   };
    // -- look for `;` termination
   (1 @ { $($fmt:tt)* }, { $($kv:tt)* }, $l:expr, $lvl:expr, $tag:expr, $msg_fmt:expr,) => {
       log!(2 @ { $($fmt)* }, { $($kv)* }, $l, $lvl, $tag, $msg_fmt)
   };
   (1 @ { $($fmt:tt)* }, { $($kv:tt)* }, $l:expr, $lvl:expr, $tag:expr, $msg_fmt:expr) => {
       log!(2 @ { $($fmt)* }, { $($kv)* }, $l, $lvl, $tag, $msg_fmt)
   };
   (1 @ { $($fmt:tt)* }, { $($kv:tt)* }, $l:expr, $lvl:expr, $tag:expr, $msg_fmt:expr, ; $($args:tt)*) => {
       log!(1 @ { $($fmt)* }, { $($kv)* }, $l, $lvl, $tag, $msg_fmt; $($args)*)
   };
   (1 @ { $($fmt:tt)* }, { $($kv:tt)* }, $l:expr, $lvl:expr, $tag:expr, $msg_fmt:expr; $($args:tt)*) => {
       log!(2 @ { $($fmt)* }, { $($kv)* }, $l, $lvl, $tag, $msg_fmt, $($args)*)
   };
    // -- must be normal argument to format string
   (1 @ { $($fmt:tt)* }, { $($kv:tt)* }, $l:expr, $lvl:expr, $tag:expr, $msg_fmt:expr, $f:tt $($args:tt)*) => {
       log!(1 @ { $($fmt)* $f }, { $($kv)* }, $l, $lvl, $tag, $msg_fmt, $($args)*)
   };
   ($l:expr, $lvl:expr, $tag:expr, $fmt:tt, $($args:tt)*) => {
       if $lvl.as_usize() <= slog::__slog_static_max_level().as_usize() {
           log!(1 @ { }, { }, $l, $lvl, $tag, $fmt; $($args)*)
       }
   };
   ($l:expr, $lvl:expr, $tag:expr, $fmt:tt) => {
       if $lvl.as_usize() <= slog::__slog_static_max_level().as_usize() {
           log!(1 @ { }, { }, $l, $lvl, $tag, $fmt; )
       }
   };
);
