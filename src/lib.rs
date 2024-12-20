pub mod simulator;

#[cfg(test)]
mod tests {
    use crate::simulator::{self, Endpoint};
    use std::io::{BufRead, BufReader, Write};

    #[test]
    fn it_works() {
        let mut client: Endpoint = Box::new(|stream| {
            let bytes_written = stream.write("hello world!\n".as_bytes()).unwrap();
            stream.flush().unwrap();

            println!("e1 wrote {bytes_written} bytes");

            let mut buf = String::new();
            BufReader::new(stream).read_line(&mut buf).unwrap();

            println!("e1 received: {}", &buf[0..buf.len() - 1]);

            assert!(buf == "goodbye\n");
        });

        let mut server: Endpoint = Box::new(|stream| {
            let mut buf = String::new();
            BufReader::new(&*stream).read_line(&mut buf).unwrap();

            println!("e2 received: {}", &buf[0..buf.len() - 1]);

            assert!(buf == "hello world!\n");

            let bytes_written = stream.write("goodbye\n".as_bytes()).unwrap();
            stream.flush().unwrap();

            println!("e2 wrote {bytes_written} bytes");
        });

        println!(
            "{:?}",
            simulator::run(
                (4..10).step_by(2),
                (20..400).step_by(100),
                3,
                &mut client,
                &mut server,
            )
        );
    }
}
