use std::io::*;
fn main() {
    let mut b = BufWriter::new(stdout());
    stdin().lines().skip(1).for_each(|s| {
        let s = s.unwrap();
        b.write('l: {
            let mut c = 0;
            let (mut start, mut end) = (s.as_ptr(), unsafe { s.as_ptr().add(s.len() - 1) });
            while start < end {
                if unsafe { *start != *end } {
                    if c + 1 == 2 {
                        break 'l b"NO\n";
                    }

                    c += 1;
                }

                unsafe {
                    start = start.add(1);
                    end = end.sub(1);
                }
            }

            b"YES\n"
        });
    });
}
