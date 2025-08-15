#[macro_export]
macro_rules! define_async_write_handler {
    ($name:ident, $ty:ty, $len:expr, $from_bytes:expr) => {
        async fn $name<F, Fut>(data: &[u8], mut f: F)
        where
            F: FnMut($ty) -> Fut,
            Fut: core::future::Future<Output = ()>,
        {
            if data.len() == $len {
                let value = $from_bytes(data);
                f(value).await;
            } else {
                warn!(
                    "[gatt] Write Event: invalid data length for {}: {:?}",
                    stringify!($ty),
                    data
                );
            }
        }
    };
}

#[macro_export]
macro_rules! define_write_handler {
    ($name:ident, $ty:ty, $len:expr, $from_bytes:expr) => {
        fn $name<F>(data: &[u8], f: F)
        where
            F: Fn($ty),
        {
            if data.len() == $len {
                let value = $from_bytes(data);
                f(value);
            } else {
                warn!(
                    "[gatt] Write Event: invalid data length for {}: {:?}",
                    stringify!($ty),
                    data
                );
            }
        }
    };
}
