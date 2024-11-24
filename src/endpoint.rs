use std::net::TcpStream;

pub struct Endpoint<'a> {
    f: Box<dyn FnMut(&mut TcpStream) -> () + Send + 'a>,
}

impl<'a> Endpoint<'a> {
    pub fn new(f: impl FnMut(&mut TcpStream) -> () + Send + 'a) -> Self {
        Endpoint { f: Box::new(f) }
    }

    pub fn exec(&mut self, stream: &mut TcpStream) {
        (self.f)(stream)
    }
}
