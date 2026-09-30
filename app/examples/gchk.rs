fn main() {
    let pack = engine::Pack::load(std::path::Path::new("assets/mowin-boris")).unwrap();
    let centre = |fno: u32| -> (i32,i32) {
        let f = pack.frame(1000, fno).unwrap();
        let img = pack.image(&f.png);
        (f.bx + f.dx + img.w as i32/2, f.by + f.dy + img.h as i32/2)
    };
    for first in [299u32, 59, 79, 154, 30, 34] {
        let len = pack.seq_len(1000, first);
        let (x0,y0) = centre(first);
        let (xl,yl) = centre(first + len - 1);
        let (fx,fy) = centre(first);
        println!("run {first} len {len}: net cycle link = ({},{})  firstcentre=({x0},{y0}) lastcentre=({xl},{yl})", fx-x0, fy-yl);
        let mut px = 0; let mut py = 0; let mut cx = 0; let mut cy = 0;
        for i in first..first+len {
            let (x,y) = centre(i);
            if i > first { cx += x - px; cy += y - py; }
            px = x; py = y;
        }
        println!("   sum of step links = ({cx},{cy})");
    }
}
