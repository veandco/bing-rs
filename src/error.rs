error_chain!{
    foreign_links {
        CellBorrowMut(::std::cell::BorrowMutError);
        Hyper(::hyper::Error);
        HyperTls(::native_tls::Error) #[cfg(feature = "rust-native-tls")];
        Io(::std::io::Error);
        SerdeJson(::serde_json::Error);
        FromUtf(::std::string::FromUtf8Error);
        Ws(::ws::Error);
    }
}
