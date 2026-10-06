// 囡囝共生 · 容器编译器 v0.3
// 五维 → 田字格容器 → BMP（零依赖，纯 math）
// 每格 4 分域：声母(左上) · 韵母(右上) · 声调(左下) · 微位(右下特征点)
// 符号 = 几何形态，绝不出现 Latin 字母

mod grid_notation;
mod registry;
use grid_notation::{Bias, decode};
use registry::{Registry, ResidenceStatus, CollideResult, YunmuTable};

const W: i32 = 1920;
const H: i32 = 1080;
const BG: u32 = 1313044; // 画布背景（深灰近黑，= 0x141414 十进制）

// ─── 几何元语 ───────────────────────────────────────────

// Canvas 封装：用 (data, width, height) 三代替代全局 W/H 常量，支持不同尺寸的画布

struct Cv { d: Vec<u32>, w: i32, h: i32 }

fn put(c: &mut Cv, x: i32, y: i32, v: u32) {
    if x >= 0 && x < c.w && y >= 0 && y < c.h {
        c.d[(y as usize) * (c.w as usize) + (x as usize)] = v;
    }
}

fn rect(c: &mut Cv, x0: i32, y0: i32, x1: i32, y1: i32, col: u32) {
    for y in y0..=y1 { for x in x0..=x1 { put(c, x, y, col); } }
}

fn line(c:  &mut Cv, x0: i32, y0: i32, x1: i32, y1: i32, col: u32) {
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut e = dx + dy;
    let mut x = x0;
    let mut y = y0;
    loop {
        put(c, x, y, col);
        if x == x1 && y == y1 { break; }
        let e2 = 2 * e;
        if e2 >= dy { e += dy; x += sx; }
        if e2 <= dx { e += dx; y += sy; }
    }
}

fn circle(c:  &mut Cv, cx: i32, cy: i32, r: i32, col: u32) {
    let mut x = r;
    let mut y = 0;
    let mut e = 1 - r;
    loop {
        put(c, cx + x, cy + y, col);
        put(c, cx - x, cy + y, col);
        put(c, cx + x, cy - y, col);
        put(c, cx - x, cy - y, col);
        put(c, cx + y, cy + x, col);
        put(c, cx - y, cy + x, col);
        put(c, cx + y, cy - x, col);
        put(c, cx - y, cy - x, col);
        y += 1;
        if e < 0 { e += 2 * y + 1; } else { x -= 1; e += 2 * (y - x) + 1; }
        if x < y { break; }
    }
}

// 菱形（对角线交叉）
fn diamond(c:  &mut Cv, cx: i32, cy: i32, r: i32, col: u32) {
    line(c, cx - r, cy, cx, cy - r, col);
    line(c, cx, cy - r, cx + r, cy, col);
    line(c, cx + r, cy, cx, cy + r, col);
    line(c, cx, cy + r, cx - r, cy, col);
}

// 三角
fn triangle(c:  &mut Cv, cx: i32, cy: i32, s: i32, col: u32) {
    line(c, cx, cy - s, cx + s, cy + s, col);
    line(c, cx + s, cy + s, cx - s, cy + s, col);
    line(c, cx - s, cy + s, cx, cy - s, col);
}

// ─── 声母编码（48 → 几何） ───────────────────────────────
// 0–3:  唇音 b p m f     → 竖条 1–4
// 4–7:  舌尖 d t n l     → 横条 1–4
// 8–11: 舌根 g k h w     → 点 1–4
// 12–15:舌面 j q x y     → 圆形
// 16–19:平舌 z c s r     → 菱形
// 20–23:扩展 ȥ ç ş ŋ    → 三角
// 24–47:大写 = 小写 + 右侧竖线（区分大小写）

fn draw_shengmu_big(c: &mut Cv, sx: i32, sy: i32, size: i32, sid: u32, col: u32) {
    // 大尺寸声母图标
    draw_shengmu(c, sx, sy, size, sid, col);
    // 加外框让它更显眼
    line(c, sx, sy, sx + size, sy, col);
    line(c, sx + size, sy, sx + size, sy + size, col);
    line(c, sx + size, sy + size, sx, sy + size, col);
    line(c, sx, sy + size, sx, sy, col);
}

// 实心竖条（2px 粗）——快 10 倍
fn solid_vbar(c: &mut Cv, cx: i32, cy: i32, h: i32, w: i32, col: u32) {
    rect(c, cx - w/2, cy - h/2, cx + w/2, cy + h/2, col);
}
fn solid_hbar(c: &mut Cv, cx: i32, cy: i32, w: i32, h: i32, col: u32) {
    rect(c, cx - w/2, cy - h/2, cx + w/2, cy + h/2, col);
}
// 实心菱形
fn solid_diamond(c: &mut Cv, cx: i32, cy: i32, r: i32, col: u32) {
    for dy in -r..=r {
        let w = r - dy.abs();
        rect(c, cx - w, cy + dy, cx + w, cy + dy, col);
    }
}
// 实心三角（正）
fn solid_triangle(c: &mut Cv, cx: i32, cy: i32, s: i32, col: u32) {
    for dy in 0..=s {
        let w = dy;
        rect(c, cx - w, cy - s + dy, cx + w, cy - s + dy, col);
    }
}
// 实心圆（中点算法填充）
fn solid_circle(c: &mut Cv, cx: i32, cy: i32, r: i32, col: u32) {
    for dy in -r..=r {
        let w = ((r * r - dy * dy) as f64).sqrt() as i32;
        rect(c, cx - w, cy + dy, cx + w, cy + dy, col);
    }
}

// ─── 笔画基元（16 基础笔画 · 纯几何实心） ──────────────────
// 统一签名: fn(canvas, x, y, w, h, col)
// (x,y) = 子格左上角, (w,h) = 子格宽高, col = 颜色
// 每笔画在子格内自适应大小，无需调用方传入尺寸

// 横 一 → 袋 → 水平实心条
fn stroke_heng(c: &mut Cv, x: i32, y: i32, w: i32, h: i32, col: u32) {
    let t = (h / 5).max(2);          // 笔画粗细
    let cy = y + h / 2;
    rect(c, x + 2, cy - t/2, x + w - 2, cy + t/2, col);
}

// 竖 丨 → 桶 → 垂直实心条（通长）
fn stroke_shu(c: &mut Cv, x: i32, y: i32, w: i32, h: i32, col: u32) {
    let t = (w / 5).max(2);
    let cx = x + w / 2;
    rect(c, cx - t/2, y + 2, cx + t/2, y + h - 2, col);
}

// 撇 丿 → 罐 → 从右上到左下的斜条（实心）
fn stroke_pie(c: &mut Cv, x: i32, y: i32, w: i32, h: i32, col: u32) {
    let t = (w / 6).max(2);
    let steps = h;
    for i in 0..=steps {
        let py = y + i;
        let px = x + w - (i * w / steps);  // 从右到左
        rect(c, px - t/2, py, px + t/2, py + 1, col);
    }
}

// 捺 ㇏ → 盆 → 从左上到右下的斜条（实心）
fn stroke_na(c: &mut Cv, x: i32, y: i32, w: i32, h: i32, col: u32) {
    let t = (w / 6).max(2);
    let steps = h;
    for i in 0..=steps {
        let py = y + i;
        let px = x + (i * w / steps);  // 从左到右
        rect(c, px - t/2, py, px + t/2, py + 1, col);
    }
}

// 点 丶 → 盂 → 小实心菱形（范围标记）
fn stroke_dian(c: &mut Cv, x: i32, y: i32, w: i32, h: i32, col: u32) {
    let r = (w / 5).max(2);
    let cx = x + w / 2;
    let cy = y + h / 2;
    solid_diamond(c, cx, cy, r, col);
}

// 提 ㇀ → 壶 → 从左下到右上的斜条（实心）
fn stroke_ti(c: &mut Cv, x: i32, y: i32, w: i32, h: i32, col: u32) {
    let t = (w / 6).max(2);
    let steps = h;
    for i in 0..=steps {
        let py = y + h - i;   // 从下到上
        let px = x + (i * w / steps);  // 从左到右
        rect(c, px - t/2, py, px + t/2, py + 1, col);
    }
}

// 横折 ㇕ → 盒 → L 形：先水平再垂直向下
fn stroke_zhe(c: &mut Cv, x: i32, y: i32, w: i32, h: i32, col: u32) {
    let t = (w / 6).max(2);
    // 横部
    rect(c, x + w/4, y + h/8, x + w - 2, y + h/8 + t, col);
    // 竖部
    rect(c, x + w - 2 - t, y + h/8, x + w - 2, y + h - 2, col);
}

// 竖钩 ㇚ → 盎 → 垂直条 + 底部钩（向左折）
fn stroke_gou(c: &mut Cv, x: i32, y: i32, w: i32, h: i32, col: u32) {
    let t = (w / 6).max(2);
    // 竖部
    rect(c, x + w/2 - t/2, y + 2, x + w/2 + t/2, y + h*3/4, col);
    // 钩部（向左折出）
    rect(c, x + w/4, y + h*3/4 - t/2, x + w/2, y + h*3/4 + t/2, col);
    rect(c, x + w/4, y + h*3/4, x + w/4 + t, y + h - 2, col);
}

// 笔画 ID → 函数指针表
type StrokeFn = fn(&mut Cv, i32, i32, i32, i32, u32);
const STROKES: [StrokeFn; 8] = [
    stroke_heng,  // 0: 横
    stroke_shu,   // 1: 竖
    stroke_pie,   // 2: 撇
    stroke_na,    // 3: 捺
    stroke_dian,  // 4: 点
    stroke_ti,    // 5: 提
    stroke_zhe,   // 6: 横折
    stroke_gou,   // 7: 竖钩
];

// 声母发音部位 → 笔画几何（满全法口腔位置编码）
// 部位决定笔画类型，声母序号决定笔画位置/细节差异
//  唇音 b p m f     → 横系 (嘴唇开合线)
//  舌尖 d t n l     → 竖系 (舌-齿接触垂线)
//  舌根 g k h W     → 折系 (舌根曲折线)
//  舌面 j q x Y     → 钩弧系 (舌面-腭弧线)
//  卷舌 z c s r     → 折回系 (舌尖反卷)
//  喉鼻 ȥ ç ş ŋ    → 复合系 (鼻道气流双通道)
// 大写 +24           → 右下角白方块附加
// 声母变长笔画序列（满全法：笔画数 = 发音复杂度的自然涌现）
//
// 笔画数量决定规则：
//   清塞音 bd g j     → 1 笔（干脆一次爆破）
//   清塞送气 pt kq    → 2 笔（爆破 + 送气尾）
//   鼻音 mn ŋ          → 2 笔（口腔爆破 + 鼻道延续）
//  擦音 f h sx        → 2 笔（摩擦面 + 出口）
//  塞擦音 zcȥ          → 3 笔（先塞后擦）
//  边音 r             → 2 笔（舌侧分开气+出气）
//  半元音WY            → 1 笔（简单气动）
//
// 声母 48 = 24 小写 + 24 大写，笔画序列相同，大写右下角加方标记

// 笔画类型常量
//   0横  1竖  2撇  3捺  4点  5提  6横折  7竖钩

// 声母笔画编码表（24 声母 × 变长序列，用静态展开）
// 编码：笔画ID*10 + 方向码(0-9)
// 每声母一个定长三元组 (a, b, c)，0xFFFFFFFF 表示无笔画

const SHENG_MU_STROKES: [(u32, u32, u32); 24] = [
    // 唇音组（塞擦+擦）
    (09, 60, 0xFFFF_FFFF),   // 0  b: 横 + 微点
    (06, 10, 0xFFFF_FFFF),   // 1  p: 横+折 + 送气（多笔画=送气尾）
    (00, 01, 0xFFFF_FFFF),   // 2  m: 双横（鼻道双线）
    (04, 10, 0xFFFF_FFFF),   // 3  f: 横 + 摩擦点
    // 舌尖组（塞+塞+鼻+边）
    (10, 70, 0xFFFF_FFFF),   // 4  d: 竖 + 横弹
    (16, 10, 0xFFFF_FFFF),   // 5  t: 竖+折 + 送气
    (10, 07, 0xFFFF_FFFF),   // 6  n: 竖 + 横鼻
    (10, 74, 0xFFFF_FFFF),   // 7  l: 竖 + 舌侧
    // 舌根组（塞+塞擦+擦+圆唇）
    (60, 40, 0xFFFF_FFFF),   // 8  g: 横折 + 腭触
    (60, 01, 0xFFFF_FFFF),   // 9  k: 横折 + 送气
    (10, 00, 0xFFFF_FFFF),   // 10 h: 竖 + 喉开
    (32, 00, 0xFFFF_FFFF),   // 11 W: 捺 + 撇圆唇
    // 舌面组（塞擦 × 4）
    (74, 10, 0xFFFF_FFFF),   // 12 j: 竖钩 + 贴腭
    (70, 01, 0xFFFF_FFFF),   // 13 q: 竖钩 + 横
    (23, 40, 0xFFFF_FFFF),   // 14 x: 撇 + 捺
    (40, 41, 0xFFFF_FFFF),   // 15 Y: 点 + 横
    // 卷舌组（塞擦+塞擦+擦+边）
    (60, 05, 0xFFFF_FFFF),   // 16 z: 横折 + 横
    (66, 00, 0xFFFF_FFFF),   // 17 c: 双横折（强化反卷）
    (13, 40, 0xFFFF_FFFF),   // 18 s: 竖 + 捺
    (02, 40, 0xFFFF_FFFF),   // 19 r: 横 + 撇
    // 喉/鼻组（擦+塞擦+擦+鼻）
    (66, 40, 0xFFFF_FFFF),   // 20 ȥ: 双横折 + 点
    (60, 41, 0xFFFF_FFFF),   // 21 ç: 横折 + 点
    (20, 70, 0xFFFF_FFFF),   // 22 ş: 撇 + 横
    (51, 10, 0xFFFF_FFFF),   // 23 ŋ: 提 + 竖
];

/// 声母序列（旧接口兼容 + 新功能兼容）
/// 返回三元组笔画编码数组
#[inline]
fn shengmu_enc(sid: u32) -> (u32, u32, u32) {
    SHENG_MU_STROKES[(sid % 24) as usize]
}

// ─── 42 色六系色彩系统 ──────────────────────────────────
// 继承 color_custom.c 命名权威：六系 × 七列 = 42 色
//   红系R(暖): 妃0 粉1 彤2 赤3 棕4 绛5 赭6
//   黄系Y(光): 缃0 金1 黄2 褐3 黧4 乌5 黑6
//   绿系G(生): 缥0 翠1 绿2 青3 苍4 黛5 玄6
//   蓝系B(冷): 素0 银1 蓝2 紫3 靛4 绀5 黯6
//   泽系S(石): 玉0 琅1 晶2 璃3 珀4 瑙5 璧6
//   光系L(韵): 曦0 辉1 霓2 旖3 靡4 暝5 黟6
//
// 声母发音部位 → 色系（满全法映射）：
//   唇音 bp m f     → B 蓝系(冷)  清塞气流从唇出 → 冷收束
//   舌尖 dt n l     → Y 黄系(光)  气流最前最亮 → 光照
//   舌根 g k h W    → G 绿系(生)  最深最稳 → 生生不息
//   舌面 j q x Y    → R 红系(暖)  舌面抬起贴腭 → 热接触
//   卷舌 z c s r    → L 光系(韵)  反卷最高最远 → 高维韵
//   喉鼻 ȥ ç ş ŋ   → S 泽系(石)  最深层鼻腔 → 石质基底

/// 从序号 (0-41) 派生 RGB：色相角 = 系*60 + 列*8，HSV→RGB
fn c42_rgb(idx: u32) -> u32 {
    let idx = idx % 42;
    let series = idx / 6;   // 0-6（第几列：0=最浅最深）
    let row = idx % 6;      // 0-5（第几系）
    // 色相：row * 51 + series * 8  (0-360 范围内，每系约 51° 宽)
    let h = (row * 51 + series * 8) as f64;
    let s = 0.75 + (series as f64) * 0.04;  // 0.75-0.99 饱和度递增
    let v = 0.95 - (series as f64) * 0.10;  // 0.95-0.45 明度递减（越深层越浓）
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r1, g1, b1) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let r = ((r1 + m) * 255.0) as u32;
    let g = ((g1 + m) * 255.0) as u32;
    let b = ((b1 + m) * 255.0) as u32;
    (r << 16) | (g << 8) | b
}

/// 声母 → 42 着色号
/// 每组 4 声母占 1 色系 × 4 列；大写（sid≥24）移列 0→3
fn shengmu_c42(sid: u32) -> u32 {
    let k = sid % 24;
    let row = k / 4;   // 0-5 = 六个发音部位 → 六系
    let col = k % 4;   // 0-3 = 组内声母 → 列位置
    let col = if sid >= 24 { col + 3 } else { col };  // 大写移列后段
    row * 6 + col
}

#[inline]
fn shengmu_color(sid: u32) -> u32 { c42_rgb(shengmu_c42(sid)) }

/// 韵母 → 42 着色号（循环取 42）
fn yunmu_c42(yid: u32) -> u32 { yid % 42 }
fn yunmu_color(yid: u32) -> u32 { c42_rgb(yunmu_c42(yid)) }

/// 声调 → 42 着色号（60 声调 / 42 色 = 每色约 1.43 步长）
fn tone_c42(t: u32) -> u32 { (t * 42 / 60) % 42 }
fn tone_color(t: u32) -> u32 { c42_rgb(tone_c42(t)) }

fn draw_shengmu(c: &mut Cv, sx: i32, sy: i32, size: i32, sid: u32, col: u32) {
    // 用笔画基元绘制声母（变长：sentinel = 0xFFFFFFFF）
    let is_upper = sid >= 24;
    let strokes = shengmu_enc(sid);

    // 笔画三元组 → 迭代（跳过 sentinel）
    let s = [strokes.0, strokes.1, strokes.2];
    let mut n = 0;
    while n < 3 && s[n] != 0xFFFF_FFFF { n += 1; }

    // 绘制笔画（第一笔占满格，后续笔逐渐缩小偏移）
    let sw = size / 2;
    let sh = size / 2;
    for i in 0..n {
        let code = s[i];
        let id = (code / 10) as usize;
        let dir = code % 10;
        let (ox, oy) = stroke_anchor(dir, sx, sy, size);
        let (dx, dy, ww, hh) = if i == 0 {
            (0i32, 0i32, sw, sh)
        } else if i == 1 {
            (sw / 4, sh / 4, sw / 2, sh / 2)
        } else {
            (sw / 3, sh / 3, sw / 2, sh / 2)
        };
        if id < STROKES.len() {
            STROKES[id](c, (ox as i32) + dx, (oy as i32) + dy, ww as i32, hh as i32, col);
        }
    }

    // 大写标记：左下角额外小点
    if is_upper {
        let d = (size / 6).max(2);
        solid_diamond(c, sx + size/4, sy + size - size/4, d, col);
    }
}

// 辅助：根据方向码计算子格锚点偏移
// 方向码：笔画ID*10 + 位置(0=左上 1=中上 2=右上 3=左中 4=中心 5=右中 6=左下 7=中下 8=右下 9=填满)
fn stroke_anchor(dir_code: u32, sx: i32, sy: i32, size: i32) -> (i32, i32) {
    match dir_code {
        0 => (sx + size/12, sy + size/12),            // 左上
        1 => (sx + size/3, sy + size/12),             // 中上
        2 => (sx + size - size/3, sy + size/12),      // 右上
        3 => (sx + size/12, sy + size/3),             // 左中
        4 => (sx + size/3, sy + size/3),              // 中心（双笔画都占中心）
        5 => (sx + size - size/3, sy + size/3),       // 右中
        6 => (sx + size/12, sy + size - size/3),      // 左下
        7 => (sx + size/3, sy + size - size/3),       // 中下
        8 => (sx + size - size/3, sy + size - size/3),// 右下
        9 => (sx, sy),                                // 填满
        _ => (sx + size/3, sy + size/3),
    }
}

// ─── 韵母编码（66 → 几何） ───────────────────────────────
// 0–5:   基础复韵 → 弧线（不同角度）
// 6–11:  合口     → 双弧
// 12–17: 卷舌     → 折线
// 18–27: 前鼻音   → 方框 + 内点数
// 28–35: 后鼻音   → 大方框
// 36–45: ia/ua组  → 梯形/领结
// 46–65: 剩余     → 复合形状

// 韵母 66 → 笔画模式编码（笔画ID*10 + 方向 + 笔画ID*10 + 方向）
// 韵母分组：
//  0–5   a o e i u ü          → 单笔画
//  6–11  ai ei ui ao ou iu    → 双笔画（复合韵）
// 12–17  ie üe er an en in    → 折类韵（含横折/竖钩/竖弯钩）
// 18–27  un ün ang eng ing ong + 2 → 复合折笔（弯钩/横折折）
// 28–35  ia ua üa io uo üo ieüo uoa → 三合韵
// 36–65  剩余 20              → 复杂韵形
#[inline]
fn yunmu_strokes(yid: u32) -> (u32, u32) {
    let k = yid;
    match k {
        // 单韵母 — 用一个笔画占满子格
        0 => (09, 00),   // a: 横 (填满)
        1 => (29, 00),   // o: 撇 (椭圆形用撇代)
        2 => (19, 00),   // e: 竖
        3 => (49, 00),   // i: 点 (居中)
        4 => (59, 00),   // u: 提
        5 => (69, 00),   // ü: 横折
        // 二合韵母 — 两个笔画左右/上下排
        6 => (20, 30),   // ai: 撇(左上) + 横(左中)
        7 => (00, 30),   // ei: 横(左上) + 竖(左中)
        8 => (10, 40),   // ui: 竖(左上) + 横折(中心)
        9 => (40, 00),   // ao: 点(左上) + 横(中上)
        10 => (50, 30),  // ou: 提(左上) + 竖(左中)
        11 => (20, 50),  // iu: 撇(左上) + 横(左上区)
        // 卷舌/鼻韵母 — 含折笔
        12 => (60, 40),  // ie: 横折(左上) + 点(中心)
        13 => (70, 30),  // üe: 竖钩(左上) + 横(左中)
        14 => (20, 00),  // er: 撇(左上) — 卷舌
        15 => (60, 00),  // an: 横折(左上)
        16 => (10, 60),  // en: 竖(左上) + 横折(左下)
        17 => (40, 10),  // in: 点(左上) + 竖(中上)
        // 鼻音韵尾 — 加竖/横折
        18 => (16, 00),  // un: 竖(左上) + 横折(中)
        19 => (06, 30),  // ün: 横(左上) + 竖钩(左中)
        20 => (60, 50),  // ang: 横折(左上) + 提(左下)
        21 => (10, 44),  // eng: 竖(左上) + 竖(中) + 点(中)
        22 => (40, 11),  // ing: 点(左上) + 点(中上) — 两点靠紧
        23 => (50, 00),  // ong: 提(左上) + 横(中上)
        24 => (24, 30),  // iana: 撇(中上) + 横(左中)
        25 => (15, 10),  // ionga: 竖(中上) + 点(中上)
        26 => (60, 60),  // uenga: 横折(左上) + 横折(左下)
        27 => (03, 40),  // uen: 横(左上) + 捺(中心)
        // 三合韵母 — 三笔画
        28 => (20, 44),  // ia: 撇(左上) + 点(中心) + 横(右下，通过第三笔画叠加)
        29 => (10, 64),  // ua: 竖(左上) + 点(中心) + 横折(右下)
        30 => (60, 04),  // üa: 横折(左上) + 点(右上) + 竖(左下)
        31 => (40, 30),  // io: 点(左上) + 横(左中)
        32 => (23, 40),  // uo: 撇(左上) + 捺(中心)
        33 => (70, 10),  // üo: 竖钩(左上) + 点(中上)
        34 => (20, 60),  // ieüo: 撇(左上) + 竖钩(左下)
        35 => (05, 40),  // uoa: 横(左上) + 提(中心)
        // 复杂韵形（复用 0~35 的变体）
        36 => (21, 00),  // 额外韵1
        37 => (12, 00),  // 额外韵2
        38 => (33, 00),  // 额外韵3: 捺+捺
        39 => (61, 00),  // 额外韵4
        40 => (07, 00),  // 额外韵5
        41 => (52, 00),  // 额外韵6
        42 => (43, 00),  // 额外韵7
        43 => (14, 00),  // 额外韵8
        44 => (35, 00),  // 额外韵9
        45 => (26, 00),  // 额外韵10
        46 => (62, 00),  // 额外韵11
        47 => (03, 30),  // 额外韵12
        48 => (71, 00),  // 额外韵13
        49 => (42, 30),  // 额外韵14
        50 => (13, 00),  // 额外韵15
        51 => (36, 00),  // 额外韵16
        52 => (54, 00),  // 额外韵17
        53 => (25, 00),  // 额外韵18
        54 => (64, 00),  // 额外韵19
        55 => (01, 40),  // 额外韵20
        56 => (72, 00),  // 额外韵21
        57 => (47, 00),  // 额外韵22
        58 => (17, 00),  // 额外韵23
        59 => (65, 00),  // 额外韵24
        60 => (30, 60),  // 额外韵25
        61 => (56, 00),  // 额外韵26
        62 => (27, 00),  // 额外韵27
        63 => (66, 00),  // 额外韵28
        64 => (04, 70),  // 额外韵29
        65 => (55, 00),  // 额外韵30
        _  => ((k % 6) * 10 + 9, ((k / 6) % 8) * 10),  // 超出 65 → 自动映射
    }
}

fn draw_yunmu(c: &mut Cv, sx: i32, sy: i32, size: u32, yid: u32, col: u32) {
    let (s0, s1) = yunmu_strokes(yid);
    let sz = size as i32;

    // 编码解析：s = 笔画ID*10 + 方向码
    let id0 = (s0 / 10) as usize;
    let dir0 = s0 % 10;
    let id1 = (s1 / 10) as usize;
    let dir1 = s1 % 10;

    let (ox0, oy0) = stroke_anchor(dir0, sx, sy, sz);
    let (ox1, oy1) = stroke_anchor(dir1, sx, sy, sz);

    let sw = sz / 2;
    let sh = sz / 2;

    // 第一笔画
    if id0 < STROKES.len() {
        STROKES[id0](c, ox0, oy0, sw, sh, col);
    }
    // 第二笔画（/ 第一笔画的变体：方向码=9 表示填满整格）
    if id1 < STROKES.len() && (id1 != id0 || dir1 != dir0) && dir1 != 0 {
        STROKES[id1](c, ox1, oy1, sw, sh, col);
    }
}

fn draw_arc(c:  &mut Cv, cx: i32, cy: i32, r: i32, degree: u32, col: u32) {
    let start = degree as f64 * std::f64::consts::PI / 180.0;
    let end = start + std::f64::consts::PI;
    let steps = 20;
    let mut prev_x = -1i32;
    let mut prev_y = -1i32;
    for i in 0..=steps {
        let t = start + (end - start) * (i as f64) / (steps as f64);
        let x = cx + (t.cos() * r as f64) as i32;
        let y = cy + (t.sin() * r as f64) as i32;
        if i > 0 {
            line(c, prev_x, prev_y, x, y, col);
        }
        prev_x = x;
        prev_y = y;
    }
}

// ─── 声调编码（60 → 几何） ───────────────────────────────
// 60 = 12 调型 × 5 调值区域 → 用 12 种方向和 5 种位置

// 声调 60 → 笔画调型（12 调型 × 5 调值位置）
// 调型笔画映射：
//   0: 横 (阴平 55 高平)
//   1: 提 (阳平 35 中升)
//   2: 竖 (上声 21 低降)
//   3: 撇 (去声 51 全降)
//   4: 横+提 (断升调)
//   5: 横折 (弯曲调)
//   6: 点 (轻声点)
//   7: 捺 (曲调)
//   8: 横+折 (复式调)
//   9: 竖钩 (回勾调)
//  10: 撇+捺 (V 形调)
//  11: 横+竖 (十字调)
fn draw_tone(c: &mut Cv, sx: i32, sy: i32, size: i32, tid: u32, col: u32) {
    let cx = sx + size / 2;
    let k = (tid % 12) as i32;
    let v = (tid / 12) as i32;  // 0~4，控制纵向位置

    let cy = sy + size / 4 + v * size / 10;  // 调值 → 纵向偏移
    let sw = size * 2 / 5;
    let sh = size * 2 / 5;

    match k {
        0 => { stroke_heng(c, cx - sw/2, cy - sh/2, sw, sh, col); }
        1 => { stroke_ti(c, cx - sw/2, cy - sh/2, sw, sh, col); }
        2 => { stroke_shu(c, cx - sw/2, cy - sh/2, sw, sh, col); }
        3 => { stroke_pie(c, cx - sw/2, cy - sh/2, sw, sh, col); }
        4 => {
            // 横+提（两段）
            let half = sw / 2;
            stroke_heng(c, cx - half, cy - sh/2, half, sh, col);
            stroke_ti(c, cx, cy - sh/2, half, sh, col);
        }
        5 => { stroke_zhe(c, cx - sw/2, cy - sh/2, sw, sh, col); }
        6 => { stroke_dian(c, cx - sw/2, cy - sh/2, sw, sh, col); }
        7 => { stroke_na(c, cx - sw/2, cy - sh/2, sw, sh, col); }
        8 => {
            // 横+折
            stroke_heng(c, cx - sw/2, cy - sh/4, sw, sh/2, col);
            stroke_zhe(c, cx - sw/2, cy, sw, sh/2, col);
        }
        9 => { stroke_gou(c, cx - sw/2, cy - sh/2, sw, sh, col); }
        10 => {
            // 撇+捺 = V 形
            stroke_pie(c, cx - sw/4, cy - sh/2, sw/2, sh, col);
            stroke_na(c, cx, cy - sh/2, sw/2, sh, col);
        }
        11 => {
            // 横+竖 = 十字
            stroke_heng(c, cx - sw/2, cy - sh/8, sw, sh/4, col);
            stroke_shu(c, cx - sw/8, cy - sh/2, sw/4, sh, col);
        }
        _ => { stroke_heng(c, cx - sw/2, cy - sh/2, sw, sh, col); }
    }
}

fn dot_lattice(c:  &mut Cv, cx: i32, cy: i32, n: i32, col: u32) {
    for dy in -1..=1 { for dx in -1..=1 { put(c, cx + dx * n / 2, cy + dy * n / 2, col); } }
}

// ─── 微位置编码（地址数值显示） ────────────────────────
// 五维地址: base = s*3960 + t*66 + y (最大 = 47*3960+59*66+65 = 186120+3894+65 = 190079)
// 微位区右侧画地址末两位（声调行末位 + 韵母末位），左侧画一个红点作为方位锚

// ─── 偏置 → 像素偏移（连接记号系统与笔画位置） ─────────
// 容器进制偏置：以 cell（子格单位偏移）为基准
//   Fine(±1) → 格内微移（笔画自身微调）
//   Grid(±1) → 跨一声调行（竖直方向）
//   Row(±1) → 跨一声母容器（水平方向 = 声母格宽）
//   Fam(±1) → 对角大跳（跨族：水平+竖直各一个声母容器）
#[inline]
fn bias_to_pixel(b: &Bias, cell: i32) -> (i32, i32) {
    let sign_v = |d: &i32| if *d < 0 { -1i32 } else { 1i32 };
    match b {
        Bias::Fine(d) => (sign_v(d) * cell, 0),
        Bias::Grid(d) => (0, sign_v(d) * cell),
        Bias::Row(d) => (sign_v(d) * cell * 2, 0),
        Bias::Fam(d) => (sign_v(d) * cell * 2, sign_v(d) * cell * 2),
    }
}

// 3×5 数字点阵字体 (0~9) — 用 7 段简化映射
// 每数字用 (dx,dy) 偏移集合表示哪些格点亮
const DIGITS: [[[bool; 3]; 5]; 10] = [
    // 0: 方框
    [[true,true,true],[true,false,true],[true,false,true],[true,false,true],[true,true,true]],
    // 1: 右竖
    [[false,true,false],[false,true,false],[false,true,false],[false,true,false],[false,true,false]],
    // 2: 上-右-中-左下-底
    [[true,true,true],[false,false,true],[true,true,true],[true,false,false],[true,true,true]],
    // 3: 上-右-中-右-底
    [[true,true,true],[false,false,true],[true,true,true],[false,false,true],[true,true,true]],
    // 4: 双竖+中横
    [[true,false,true],[true,false,true],[true,true,true],[false,false,true],[false,false,true]],
    // 5: 上-左-中-右-底
    [[true,true,true],[true,false,false],[true,true,true],[false,false,true],[true,true,true]],
    // 6: 上-左-中-双竖-底
    [[true,true,true],[true,false,false],[true,true,true],[true,false,true],[true,true,true]],
    // 7: 上-右竖
    [[true,true,true],[false,false,true],[false,false,true],[false,false,true],[false,false,true]],
    // 8: 全框+中
    [[true,true,true],[true,false,true],[true,true,true],[true,false,true],[true,true,true]],
    // 9: 上-双竖-中-右-底（右下空）
    [[true,true,true],[true,false,true],[true,true,true],[false,false,true],[true,true,true]],
];

fn draw_wz(c: &mut Cv, sx: i32, sy: i32, size: i32, s: u32, y: u32, t: u32, col: u32) {
    // 容器进制三元组地址：(s, t, y) 各取个位
    let ds = (s % 10) as usize;
    let dt = (t % 10) as usize;
    let dy = (y % 10) as usize;

    // 点阵绘制参数
    let cell = (size / 10).max(2);
    let dw = 3 * cell;
    let dh = 5 * cell;
    let gap = cell;

    // 三数字横向排列：s在左、t在中、y在右
    let total_w = 3 * dw + 2 * gap;
    let start_x = sx + (size - total_w) / 2;
    let start_y = sy + size - dh - 2;

    draw_digit(c, start_x, start_y, cell, ds, col);
    draw_digit(c, start_x + dw + gap, start_y, cell, dt, col);
    draw_digit(c, start_x + 2 * (dw + gap), start_y, cell, dy, col);

    // 左上角锚点（微位标记）
    rect(c, sx + 1, sy + 1, sx + 3, sy + 3, c42_rgb(3));
}

// 在 (x,y) 位置画一个 3×5 点阵数字，占用 width=3*cell height=5*cell
fn draw_digit(c: &mut Cv, x: i32, y: i32, cell: i32, d: usize, col: u32) {
    if d >= 10 { return; }
    for row in 0..5i32 {
        for col_idx in 0..3i32 {
            if DIGITS[d][row as usize][col_idx as usize] {
                rect(c,
                    x + col_idx * cell + 1,
                    y + row * cell + 1,
                    x + (col_idx + 1) * cell - 1,
                    y + (row + 1) * cell - 1,
                    col);
            }
        }
    }
}

// ─── 粗线（2像素）用于容器边框 ─────────────────────────────

fn thick_line(c: &mut Cv, x0: i32, y0: i32, x1: i32, y1: i32, col: u32, w: i32) {
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    if dx >= dy {
        // 水平走向：沿 Y 偏移画 w 条平行线
        for o in -w/2..=w/2 {
            line(c, x0, y0 + o, x1, y1 + o, col);
        }
    } else {
        for o in -w/2..=w/2 {
            line(c, x0 + o, y0, x1 + o, y1, col);
        }
    }
}

// ─── 田字格容器 ───────────────────────────────────────────

// C_ID 已废弃 — 颜色由 c42_rgb() 从 42 色六系派生

fn draw_container_mono(canvas:  &mut Cv, ox: i32, oy: i32, cw: i32, ch: i32,
                        sid: u32, yid: u32, tid: u32, mx: u32, my: u32, fg: u32) {
    // 田字格外框 2px —— 用前景色
    thick_line(canvas, ox, oy, ox + cw, oy, fg, 2);
    thick_line(canvas, ox + cw, oy, ox + cw, oy + ch, fg, 2);
    thick_line(canvas, ox + cw, oy + ch, ox, oy + ch, fg, 2);
    thick_line(canvas, ox, oy + ch, ox, oy, fg, 2);
    // 十字分割 1px
    line(canvas, ox + cw / 2, oy, ox + cw / 2, oy + ch, fg);
    line(canvas, ox, oy + ch / 2, ox + cw, oy + ch / 2, fg);

    let half_w = cw / 2;
    let half_h = ch / 2;

    draw_shengmu(canvas, ox + 2, oy + 2, half_w - 4, sid, fg);
    draw_yunmu(canvas, ox + half_w + 1, oy + 2, (half_w - 4) as u32, yid, fg);
    draw_tone(canvas, ox + 2, oy + half_h + 1, half_w - 4, tid, fg);
    draw_wz(canvas, ox + half_w + 1, oy + half_h + 1, half_w - 4, sid, yid, tid, fg);
}

fn draw_container(canvas: &mut Cv, ox: i32, oy: i32, cw: i32, ch: i32,
                  sid: u32, yid: u32, tid: u32, _dummy_mx: u32, _dummy_my: u32) {
    let half_w = cw / 2;
    let half_h = ch / 2;
    // 田字格框（42 色系第41号 = 光系第7列 = 黟，最深最远作框）
    let frame_col = c42_rgb(41);
    rect(canvas, ox, oy, ox + cw, oy + 1, frame_col);
    rect(canvas, ox, oy + ch - 1, ox + cw, oy + ch, frame_col);
    rect(canvas, ox, oy, ox + 1, oy + ch, frame_col);
    rect(canvas, ox + cw - 1, oy, ox + cw, oy + ch, frame_col);
    // 中线（第28号 = 蓝系中间 = 紫，冷色调做内部分隔）
    let mid_col = c42_rgb(28);
    let mid_y = oy + half_h;
    for x in ox..ox + cw { put(canvas, x, mid_y, mid_col); }
    let mid_x = ox + half_w;
    for y in oy..oy + ch { put(canvas, mid_x, y, mid_col); }

    let col_s = shengmu_color(sid);
    let col_y = yunmu_color(yid);
    let col_t = tone_color(tid);

    draw_shengmu(canvas, ox + 2, oy + 2, half_w - 4, sid, col_s);
    draw_yunmu(canvas, ox + half_w + 1, oy + 2, (half_w - 4) as u32, yid, col_y);
    draw_tone(canvas, ox + 2, oy + half_h + 1, half_w - 4, tid, col_t);
    draw_wz(canvas, ox + half_w + 1, oy + half_h + 1, half_w - 4, sid, yid, tid, c42_rgb(0));
}

// ─── 偏置驱动笔画位置 · 造字核心 ───────────────────────
// 偏置链解码后叠加到对应笔画区：作用于三区（声母/韵母/声调），
// 偏置后相应笔画偏移 ±cell、±2cell，实现"同一个声符偏置出多个字形"。
// 效果：标准笔画 + 蓝色偏置笔画 + 红色连接箭头。
fn draw_container_biased(
    canvas: &mut Cv, ox: i32, oy: i32, cw: i32, ch: i32,
    sid: u32, yid: u32, tid: u32,
    biases_s: &[Bias], biases_y: &[Bias], biases_t: &[Bias],
    col: u32,
) {
    let half_w = cw / 2;
    let h_w = half_w - 4;

    // 无条件画标准容器（标准笔画 + 田字格框 + 微位地址）
    draw_container(canvas, ox, oy, cw, ch, sid, yid, tid, 0, 0);

    // 偏置像素（以 h_w/8 为单位 cell，让 ±1cell ~ 清晰且不溢出子格）
    let cell = (h_w / 8).max(2);

    let bias_col = c42_rgb(20);  // 偏置笔画固定第20号色（绿系正中）
    let arrow_col = c42_rgb(3);  // 箭头固定第3号色（红系第4列=赤色系）

    // ─── 偏置声母笔画 ───
    if !biases_s.is_empty() {
        let (mut px, mut py) = (0i32, 0i32);
        for b in biases_s { let (dx, dy) = bias_to_pixel(b, cell); px += dx; py += dy; }
        let bx = ox + 2 + px;
        let by = oy + 2 + py;
        // 越界保护
        if bx >= ox && by >= oy && bx + h_w <= ox + half_w && by + h_w <= oy + ch/2 {
            let strokes = shengmu_enc(sid);
            let ss = [strokes.0, strokes.1, strokes.2];
            let mut sn = 0;
            while sn < 3 && ss[sn] != 0xFFFF_FFFF { sn += 1; }
            if sn >= 1 { let id0 = (ss[0] / 10) as usize; if id0 < STROKES.len() { STROKES[id0](canvas, bx, by, h_w, h_w, bias_col); } }
            if sn >= 2 { let id1 = (ss[1] / 10) as usize; if id1 < STROKES.len() { STROKES[id1](canvas, bx + h_w/4, by + h_w/4, h_w/2, h_w/2, bias_col); } }
            if sn >= 3 { let id2 = (ss[2] / 10) as usize; if id2 < STROKES.len() { STROKES[id2](canvas, bx + h_w/3, by + h_w/3, h_w/2, h_w/2, bias_col); } }
        }
        // 原始中点 → 偏置后中点箭头
        let cx0 = ox + 2 + h_w/2;
        let cy0 = oy + 2 + h_w/2;
        let cx1 = cx0 + px;
        let cy1 = cy0 + py;
        thick_line(canvas, cx0, cy0, cx1, cy1, arrow_col, 2);
    }

    // ─── 偏置韵母笔画 ───
    if !biases_y.is_empty() {
        let (mut px, mut py) = (0i32, 0i32);
        for b in biases_y { let (dx, dy) = bias_to_pixel(b, cell); px += dx; py += dy; }
        let bx = ox + half_w + 1 + px;
        let by = oy + 2 + py;
        if bx + h_w <= ox + cw && by + h_w <= oy + ch/2 {
            let (s0, s1) = yunmu_strokes(yid);
            let id0 = (s0 / 10) as usize;
            if id0 < STROKES.len() { STROKES[id0](canvas, bx, by, h_w, h_w, bias_col); }
            let id1 = (s1 / 10) as usize;
            if id1 < STROKES.len() { STROKES[id1](canvas, bx + h_w/4, by + h_w/4, h_w/2, h_w/2, bias_col); }
        }
        let cx0 = ox + half_w + 1 + h_w/2;
        let cy0 = oy + 2 + h_w/2;
        thick_line(canvas, cx0, cy0, cx0 + px, cy0 + py, arrow_col, 2);
    }

    // ─── 偏置声调笔画 ───
    if !biases_t.is_empty() {
        let (mut px, mut py) = (0i32, 0i32);
        for b in biases_t { let (dx, dy) = bias_to_pixel(b, cell); px += dx; py += dy; }
        let bx = ox + 2 + px;
        let by = oy + ch/2 + 1 + py;
        if bx + h_w <= ox + half_w && by + h_w <= oy + ch {
            // 声调笔画按调型笔画重画
            let k = (tid % 12) as i32;
            let sw = h_w;
            match k {
                0 => stroke_heng(canvas, bx, by, sw, sw, bias_col),
                1 => stroke_ti(canvas, bx, by, sw, sw, bias_col),
                2 => stroke_shu(canvas, bx, by, sw, sw, bias_col),
                3 => stroke_pie(canvas, bx, by, sw, sw, bias_col),
                5 => stroke_zhe(canvas, bx, by, sw, sw, bias_col),
                6 => stroke_dian(canvas, bx, by, sw, sw, bias_col),
                7 => stroke_na(canvas, bx, by, sw, sw, bias_col),
                9 => stroke_gou(canvas, bx, by, sw, sw, bias_col),
                _ => stroke_heng(canvas, bx, by, sw, sw, bias_col),
            }
        }
        let cx0 = ox + 2 + h_w/2;
        let cy0 = oy + ch/2 + 1 + h_w/2;
        thick_line(canvas, cx0, cy0, cx0 + px, cy0 + py, arrow_col, 2);
    }
}

// ─── 纯 Rust PNG 编码（零依赖，deflate + CRC32 手算）─────────────────

fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            if crc & 1 != 0 { crc = 0xEDB8_8320 ^ (crc >> 1); }
            else { crc >>= 1; }
        }
    }
    crc ^ 0xFFFF_FFFF
}

fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    let m = 65_521u32;
    for &byte in data {
        a = (a + byte as u32) % m;
        b = (b + a) % m;
    }
    (b << 16) | a
}

// PNG zrle：每行加 0x00 filter tag，然后 zlib Header + Adler32 校验
// 这是一个"存货式" zrl 编码：直接把所有行数据拼起来不做真正的 LZ77 压缩，
// 用 "stored blocks" (BTYPE=00) 输出未压缩内容，PNG 标准允许这种做法。
fn write_png_canvas(c: &Cv, out: &str) {
    let w = c.w as u32;
    let h = c.h as u32;
    let raw_row_size = 1 + w * 3u32; // filter tag + RGB
    let raw_total = raw_row_size * h;

    // 拼 raw RGB（每行前补 0x00 = no filter）
    let mut raw: Vec<u8> = Vec::with_capacity(raw_total as usize);
    for y in (0..h as i32).rev() {
        raw.push(0x00); // filter: None
        for x in 0..c.w {
            let px = c.d[(y as usize) * (c.w as usize) + (x as usize)];
            raw.push((px & 0xFF) as u8);         // R
            raw.push(((px >> 8) & 0xFF) as u8);  // G
            raw.push(((px >> 16) & 0xFF) as u8); // B
        }
    }

    // ── zlib stream ──
    let mut zlib: Vec<u8> = Vec::new();
    zlib.push(0x78); // CMF
    zlib.push(0x01); // FLG (no dict, no compression)

    // stored blocks (BTYPE=00 = no compression)
    // 每个 block：1 byte header (final=1 if last, BTYPE=00) + LEN(2) + NLEN(2) + data
    const MAX_BLOCK: u16 = 65_535;
    let mut offset = 0;
    while offset < raw.len() {
        let remain = raw.len() - offset;
        let blen: usize = if remain > MAX_BLOCK as usize { MAX_BLOCK as usize } else { remain };
        let is_last = (offset + blen) >= raw.len();
        zlib.push(if is_last { 0x01 } else { 0x00 }); // final=1 + BTYPE=00
        zlib.push((blen & 0xFF) as u8);
        zlib.push(((blen >> 8) & 0xFF) as u8);
        let nlen = !blen as u16;
        zlib.push((nlen & 0xFF) as u8);
        zlib.push(((nlen >> 8) & 0xFF) as u8);
        zlib.extend_from_slice(&raw[offset..offset + blen]);
        if is_last { break; } // 防止刚刚好的边界再开空 block
        offset += blen;
    }

    let adler = adler32(&raw);
    zlib.extend_from_slice(&adler.to_be_bytes());

    // ── PNG chunks ──
    let mut png: Vec<u8> = Vec::new();
    // PNG signature
    png.extend_from_slice(&[137u8, 80, 78, 71, 13, 10, 26, 10]);

    // IHDR
    let mut ihdr: Vec<u8> = Vec::with_capacity(13);
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.push(8);   // bit depth
    ihdr.push(2);   // color type: RGB
    ihdr.push(0);   // compression
    ihdr.push(0);   // filter
    ihdr.push(0);   // interlace
    let mut ihdr_full: Vec<u8> = Vec::new();
    ihdr_full.extend_from_slice(&(ihdr.len() as u32 - 0).to_be_bytes());
    // 修正：长度不含 type 和 CRC 自身
    let ihdr_chunk_len: u32 = 13;
    let mut ihdr_full: Vec<u8> = Vec::new();
    ihdr_full.extend_from_slice(&ihdr_chunk_len.to_be_bytes());
    ihdr_full.extend_from_slice(b"IHDR");
    ihdr_full.extend_from_slice(&ihdr);
    let crc = crc32(&ihdr_full[4..]); // CRC over type + data
    ihdr_full.extend_from_slice(&crc.to_be_bytes());
    png.extend_from_slice(&ihdr_full);

    // IDAT
    let mut idat_crc_data: Vec<u8> = Vec::new();
    idat_crc_data.extend_from_slice(b"IDAT");
    idat_crc_data.extend_from_slice(&zlib);
    let mut idat_full: Vec<u8> = Vec::new();
    idat_full.extend_from_slice(&(zlib.len() as u32).to_be_bytes());
    idat_full.extend_from_slice(&idat_crc_data);
    let crc = crc32(&idat_crc_data);
    idat_full.extend_from_slice(&crc.to_be_bytes());
    png.extend_from_slice(&idat_full);

    // IEND
    let iend_type = b"IEND";
    let crc_iend = crc32(iend_type);
    png.extend_from_slice(&0u32.to_be_bytes()); // length = 0
    png.extend_from_slice(iend_type);
    png.extend_from_slice(&crc_iend.to_be_bytes());

    std::fs::write(out, &png).ok();
}

fn write_bmp_canvas(c: &Cv, out: &str) {
    let w = c.w as usize;
    let h = c.h as usize;
    let rs = (w * 3 + 3) & !3;
    let mut buf: Vec<u8> = Vec::with_capacity(54 + rs * h);

    buf.extend_from_slice(b"BM");
    let fsize: u32 = 54 + (rs as u32) * (h as u32);
    buf.extend_from_slice(&fsize.to_le_bytes());
    buf.extend_from_slice(&[0u8; 4]);
    buf.extend_from_slice(&54u32.to_le_bytes());
    buf.extend_from_slice(&40u32.to_le_bytes());
    buf.extend_from_slice(&(w as i32).to_le_bytes());
    buf.extend_from_slice(&(h as i32).to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes());
    buf.extend_from_slice(&24u16.to_le_bytes());
    buf.extend_from_slice(&0u32.to_le_bytes());
    buf.extend_from_slice(&(rs as u32 * h as u32).to_le_bytes());
    buf.extend_from_slice(&2835i32.to_le_bytes());
    buf.extend_from_slice(&2835i32.to_le_bytes());
    buf.extend_from_slice(&0u32.to_le_bytes());
    buf.extend_from_slice(&0u32.to_le_bytes());

    for yy in (0..h as i32).rev() {
        for x in 0..w as i32 {
            let c2 = c.d[(yy as usize * w + x as usize)];
            buf.push((c2 & 0xFF) as u8);
            buf.push(((c2 >> 8) & 0xFF) as u8);
            buf.push(((c2 >> 16) & 0xFF) as u8);
        }
        for _ in 0..(rs - w * 3) { buf.push(0); }
    }

    std::fs::write(out, &buf).ok();
}

// ─── Main ───────────────────────────────────────────────

fn draw_label(canvas:  &mut Cv, x: i32, y: i32, s: u32, yy: u32, t: u32, mx: u32, my: u32) {
    // 用简单象形数字（矩形柱状）标注参数
    let col = c42_rgb(24);
    let cx = x + 20;

    // 声母序号 — 短竖条堆叠
    for i in 0..(s % 8) as i32 {
        line(canvas, cx, y + i * 6, cx + 30, y + i * 6, col);
    }
    // 韵母序号 — 点列
    for i in 0..(yy % 8) as i32 {
        put(canvas, cx + 35 + i * 4, y + 10, col);
        put(canvas, cx + 36 + i * 4, y + 10, col);
    }
    // 声调序号 — 横条
    for i in 0..(t % 8) as i32 {
        line(canvas, cx + 65, y + i * 5 + 2, cx + 100, y + i * 5 + 2, col);
    }
}

fn draw_row(sid: u32, out: &str) {
    // 行视图：每容器 120×120，真正能看清
    // X轴=韵母取前12列，Y轴=声调取前10行 → 120音节样品
    let cell = 90i32;
    let cols = 12i32;
    let rows = 10i32;
    let label_w = 120i32;     // 左侧声母标注
    let title_h = 50i32;      // 顶部韵母标注
    let margin = 8i32;
    let pix_w = cell;
    let pix_h = cell;
    let cw = cell - 4;         // 容器实际大小
    let ch = cell - 4;
    let canvas_w = label_w + cols * pix_w + margin * 2;
    let canvas_h = title_h + rows * pix_h + margin * 2;

    let mut cv = Cv { d: vec![BG; (canvas_w * canvas_h) as usize], w: canvas_w, h: canvas_h };

    // 声母大图标（左侧）
    draw_shengmu_big(&mut cv, 10, title_h + rows * pix_h / 2 - 45, 90, sid, c42_rgb(8));

    // 韵母列标注条（顶部 0–11 号）
    for i in 0..cols {
        let px = label_w + i * pix_w;
        // 用韵母几何小图标做列头
        draw_yunmu(&mut cv, px + pix_w/2 - 12, 4, 24, i as u32, c42_rgb(32));
        line(&mut cv, px, title_h, px, title_h + rows * pix_h, c42_rgb(40));
    }
    line(&mut cv, label_w + cols * pix_w, title_h, label_w + cols * pix_w, title_h + rows * pix_h, c42_rgb(40));

    // 声调行标注条（左侧）
    for j in 0..rows {
        let py = title_h + j * pix_h;
        // 用声调几何做行头
        draw_tone(&mut cv, 2, py + 2, label_w - 8, j as u32, c42_rgb(32));
        line(&mut cv, label_w, py, label_w + cols * pix_w, py, c42_rgb(40));
    }
    line(&mut cv, label_w, title_h + rows * pix_h, label_w + cols * pix_w, title_h + rows * pix_h, c42_rgb(40));
    // 声母底部横线
    line(&mut cv, 0, title_h, label_w, title_h, c42_rgb(37));

    // 画中容器
    let mut drawn = 0u32;
    for yy in 0..12u32 {
        for t in 0..10u32 {
            let ox = label_w + yy as i32 * pix_w + 2;
            let oy = title_h + t as i32 * pix_h + 2;
            let mx = ((sid * 31 + yy * 10 + t * 7) % 10) as u32;
            let my = ((sid * 13 + yy * 7 + t * 11) % 10) as u32;
            draw_container(&mut cv, ox, oy, cw, ch, sid, yy, t, mx, my);
            drawn += 1;
        }
    }

    write_bmp_canvas(&cv, out);
    // 同名 PNG（直接可被妙手预览）
    let png_path = out.replace(".bmp", ".png");
    write_png_canvas(&cv, &png_path);
    let non_bg = cv.d.iter().filter(|&&c| c != BG).count();
    println!("行视图 声母#{}({}) {}音节 画布:{}x{} 像素:{}({:.1}%) → {}",
             sid, ['b','p','m','f','d','t','n','l','g','k','h','w',
                   'j','q','x','y','z','c','s','r','ȥ','ç','ş','ņ',
                   'B','P','M','F','D','T','N','L','G','K','H','W',
                   'J','Q','X','Y','Z','C','S','R','Ȥ','Ç','Ş','Ņ'][sid as usize],
             drawn, canvas_w, canvas_h,
             non_bg, non_bg as f64 / (canvas_w * canvas_h) as f64 * 100.0, png_path);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // ── 特殊命令：export-reg（全量导出注册表 JSON）──
    if args.len() > 1 && args[1] == "export-reg" {
        let out = args.get(2).map(|s| s.as_str())
            .unwrap_or("/mnt/data/catpaw/home/workspace/硅碳心源-果套循因/toolchain/tier1/glyph_registry.json");
        let mut file = std::fs::File::create(out).expect("cannot create file");
        use std::io::Write;
        // 容量从 toml 读取，不自创
        let toml_path = "/mnt/data/catpaw/home/workspace/硅碳心源-果套循因/toolchain/tier1/glyph_compiler/registry.toml";
        let cap = registry::Capacity::from_toml(toml_path);
        let s_cap = cap.shengmu_cap;
        let y_cap = cap.yunmu_cap;
        let t_cap = cap.tone_cap;
        // 行格式：每行 {"base":N,"s":N,"y":N,"t":N,"strokes":[[type,dir,x,y],...]}
        let mut count = 0;
        for s in 0..s_cap {
            for t in 0..t_cap {
                for y in 0..y_cap {
                    // 容器进制：地址 = (声母s, 声调t, 韵母y) 三元组，不用乘法
                    let strokes_s = shengmu_enc(s);
                    let (y0, y1) = yunmu_strokes(y);
                    let tid = t % 12;
                    let ss = [strokes_s.0, strokes_s.1, strokes_s.2];
                    let mut sn = 0;
                    while sn < 3 && ss[sn] != 0xFFFF_FFFF { sn += 1; }
                    write!(file, "{{\"s\":{},\"t\":{},\"y\":{},\"bx\":[", s, t, y).unwrap();
                    // 声母笔画（变长 1-3 笔，不足补零占位）
                    for i in 0..3 {
                        if i > 0 { write!(file, ",").unwrap(); }
                        if i < sn {
                            write!(file, "[{},{},{},{}]", ss[i]/10, ss[i]%10,
                                (ss[i]%10) % 3, (ss[i]%10) / 3).unwrap();
                        } else {
                            write!(file, "[0,0,0,0]").unwrap();
                        }
                    }
                    write!(file, "],\"yx\":[").unwrap();
                    // 韵母笔画
                    write!(file, "[{},{},{},{}]", y0/10, y0%10,
                        (y0%10) % 3, (y0%10) / 3).unwrap();
                    write!(file, ",[{},{},{},{}]", y1/10, y1%10,
                        (y1%10) % 3, (y1%10) / 3).unwrap();
                    write!(file, "],\"tx\":[").unwrap();
                    // 声调笔画 (调型=笔画ID, 调值=方向, 调型作为type)
                    let ttype = tid as u32;
                    let tdir = (t / 12) as u32;
                    write!(file, "[{},{},0,{}]", ttype, tdir, tdir).unwrap();
                    write!(file, "]}}\n").unwrap();
                    count += 1;
                }
            }
        }
        println!("glyph_registry 导出完成: {} 音节 → {}", count, out);
        return;
    }

    // ── 特殊命令：lookup（韵母符号 → 地址查询）──
    if args.len() > 2 && args[1] == "lookup" {
        let toml_path = "/mnt/data/catpaw/home/workspace/硅碳心源-果套循因/toolchain/tier1/glyph_compiler/registry.toml";
        let table = registry::YunmuTable::from_toml(toml_path);
        let symbol = &args[2];
        let idx = table.symbol_to_index(symbol);
        if idx == 0xFFFFFFFF {
            println!("符号 \"{}\" 不在韵母符号表中", symbol);
            println!("  符号总数: {} (toml 加载)", table.len());
        } else {
            println!("符号: {}", symbol);
            println!("  地址(index): {}", idx);
            println!("  归属: {}", table.owner(symbol));
            println("  符号总数: {}", table.len());
        }
        return;
    }

    // ── 特殊命令：table-info（符号表统计）──
    if args.len() > 1 && args[1] == "table-info" {
        let toml_path = "/mnt/data/catpaw/home/workspace/硅碳心源-果套循因/toolchain/tier1/glyph_compiler/registry.toml";
        let table = registry::YunmuTable::from_toml(toml_path);
        println!("韵母符号表统计");
        println!("  符号总数: {}", table.len());
        // 列出前10个符号示例
        let show = 10.min(table.len() as usize);
        println!("  前{}个符号:", show);
        for i in 0..show {
            let sym = table.index_to_symbol(i as u32);
            println!("    [{}] {} ({})", i, sym, table.owner(sym));
        }
        return;
    }

    if args.len() > 1 && args[1] == "one" {
        let s: u32 = args.get(2).and_then(|x| x.parse().ok()).unwrap_or(0);
        let yy: u32 = args.get(3).and_then(|x| x.parse().ok()).unwrap_or(0);
        let t: u32 = args.get(4).and_then(|x| x.parse().ok()).unwrap_or(0);
        let big_w = 800i32;
        let big_h = 800i32;
        let mut cv = Cv { d: vec![16777215; (big_w * big_h) as usize], w: big_w, h: big_h };

        let mx = ((s * 31 + yy * 17 + t * 7) % 10) as u32;
        let my = ((s * 13 + yy * 7 + t * 11) % 10) as u32;

        // 浅色背景，深色笔画，最大对比度
        let fg = c42_rgb(32); // 第32号 = 绿系最深处玄色
        let m = 40i32;
        draw_container_mono(&mut cv, m, m, big_w - 2*m, big_h - 2*m, s, yy, t, mx, my, fg);

        let out_bmp = format!("/mnt/data/catpaw/home/workspace/硅碳心源-果套循因/toolchain/tier1/mqf_glyph_one_s{}_y{}_t{}.bmp", s, yy, t);
        write_bmp_canvas(&cv, &out_bmp);
        let out_png = format!("/mnt/data/catpaw/home/workspace/硅碳心源-果套循因/toolchain/tier1/mqf_glyph_one_s{}_y{}_t{}.png", s, yy, t);
        write_png_canvas(&cv, &out_png);

        let drawn = cv.d.iter().filter(|&&c| c != 16777215).count();
        println!("单音节: s={} y={} t={} mx={} my={} → {} 笔画像素:{} ({:.1}%)",
                 s, yy, t, mx, my, out_png, drawn, drawn as f64 / (big_w * big_h) as f64 * 100.0);
    } else if args.len() > 1 && args[1] == "row" {
        let sid: u32 = args.get(2).and_then(|x| x.parse().ok()).unwrap_or(0);
        let out = format!("/mnt/data/catpaw/home/workspace/硅碳心源-果套循因/toolchain/tier1/mqf_glyph_row_s{}.bmp", sid);
        draw_row(sid, &out);
    } else if args.len() > 1 && args[1] == "bias" {
        // 偏置可视化：bias s y t ±1.-2 1±.±3 ...
        // 把偏置记号的每一步画成从中心辐射的彩色条带
        let s: u32 = args.get(2).and_then(|x| x.parse().ok()).unwrap_or(0);
        let yy: u32 = args.get(3).and_then(|x| x.parse().ok()).unwrap_or(0);
        let t: u32 = args.get(4).and_then(|x| x.parse().ok()).unwrap_or(0);

        // 收集记号参数（第5个参数起）
        let mut notations = Vec::new();
        for i in 5..args.len() {
            notations.push(args[i].clone());
        }
        if notations.is_empty() {
            notations.push("+1".to_string());
        }

        let big_w = 800i32;
        let big_h = 800i32;
        const BG_DARK: u32 = 1118481; // 深灰背景
        let mut cv = Cv { d: vec![BG_DARK; (big_w * big_h) as usize], w: big_w, h: big_h };

        let m = 60i32;
        let cont_w = big_w - 2 * m;
        let cont_h = big_h - 2 * m - 80;
        // 先画容器本体（彩色）
        draw_container(&mut cv, m, m, cont_w, cont_h, s, yy, t,
                       ((s*31+yy*17+t*7)%10) as u32,
                       ((s*13+yy*7+t*11)%10) as u32);

        // 用 grid_notation::decode 正式解析记号
        let cx = m + cont_w / 2;
        let cy = m + cont_h / 2;
        let cols: [u32; 6] = [c42_rgb(0), c42_rgb(7), c42_rgb(14), c42_rgb(21), c42_rgb(28), c42_rgb(35)];

        // 量级 → 像素方向/长度映射（负=左/上，正=右/下）
        // Fine(±1)：水平微移 ±12px（音节内核微调）
        // Grid(±66)：水平中移 cont_w/6（偏韵母维）
        // Row(±3960)：垂直中移 cont_h/6（偏声母维）
        // Fam(±1)：垂直大跳 cont_h/4（跨族）
        for (idx, nota) in notations.iter().enumerate() {
            let col = cols[idx % 6];
            let biases = decode(nota);
            let mut x = cx;
            let mut y = cy;

            for b in &biases {
                let (dx, dy) = match b {
                    Bias::Fine(d) => (d * 12, 0),
                    Bias::Grid(d) => {
                        let sign = if *d < 0 { -1 } else { 1 };
                        (sign * cont_w / 6, 0)
                    }
                    Bias::Row(d) => {
                        let sign = if *d < 0 { -1 } else { 1 };
                        (0, sign * cont_h / 6)
                    }
                    Bias::Fam(d) => {
                        let sign = if *d < 0 { -1 } else { 1 };
                        (0, sign * cont_h / 4)
                    }
                };
                let nx = x + dx;
                let ny = y + dy;
                thick_line(&mut cv, x, y, nx, ny, col, 3);
                // 端点方块
                rect(&mut cv, nx-3, ny-3, nx+3, ny+3, col);
                x = nx;
                y = ny;
            }
            // 最终落点画实心圆
            let r = 12i32;
            for dy in -r..=r {
                let w = ((r*r - dy*dy) as f64).sqrt() as i32;
                rect(&mut cv, x - w, y + dy, x + w, y + dy, col);
            }
        }

        let nota_str = notations.join(" ");
        let out = format!("/mnt/data/catpaw/home/workspace/硅碳心源-果套循因/toolchain/tier1/mqf_glyph_bias_s{}_y{}_t{}_{}.bmp", s, yy, t, nota_str.replace([' ', '.', '±', '+', '-'], "_"));
        write_bmp_canvas(&cv, &out);

        let drawn = cv.d.iter().filter(|&&c| c != BG_DARK).count();
        println!("偏置: s={} y={} t={} 记号:[{}] → {} 像素({:.1}%)",
                 s, yy, t, nota_str, out, drawn as f64 / (big_w * big_h) as f64 * 100.0);
    } else if args.len() > 2 && args[1] == "biased" {
        let s: u32 = args.get(2).and_then(|x| x.parse().ok()).unwrap_or(0);
        let yy: u32 = args.get(3).and_then(|x| x.parse().ok()).unwrap_or(0);
        let t: u32 = args.get(4).and_then(|x| x.parse().ok()).unwrap_or(0);
        let mut bs_s: Vec<Bias> = Vec::new();
        let mut bs_y: Vec<Bias> = Vec::new();
        let mut bs_t: Vec<Bias> = Vec::new();
        for i in 5..args.len() {
            let a = &args[i];
            let (prefix, nota) = if a.contains(':') {
                let p: Vec<&str> = a.splitn(2, ':').collect();
                (p[0], p[1])
            } else { ("*", a.as_str()) };
            let decoded = decode(nota);
            match prefix {
                "S" => bs_s.extend(decoded),
                "Y" => bs_y.extend(decoded),
                "T" => bs_t.extend(decoded),
                _ => { bs_s.extend(decoded.clone()); bs_y.extend(decoded.clone()); bs_t.extend(decoded); }
            }
        }
        if bs_s.is_empty() && bs_y.is_empty() && bs_t.is_empty() {
            bs_s = decode("+1"); bs_y = decode("-1"); bs_t = decode("±2");
        }
        let big_w = 800i32; let big_h = 800i32; let m = 60;
        const BG_DARK: u32 = 1118481;
        let mut cv = Cv { d: vec![BG_DARK; (big_w * big_h) as usize], w: big_w, h: big_h };
        draw_container_biased(&mut cv, m, m, big_w - 2*m, big_h - 2*m, s, yy, t, &bs_s, &bs_y, &bs_t, c42_rgb(0));
        let out = format!("/mnt/data/catpaw/home/workspace/硅碳心源-果套循因/toolchain/tier1/mqf_biased_s{}_y{}_t{}.bmp", s, yy, t);
        write_bmp_canvas(&cv, &out);
        let drawn = cv.d.iter().filter(|&&c| c != BG_DARK).count();
        println!("偏置造字 s={} y={} t={} → {} 像素({:.1}%)", s, yy, t, out, drawn as f64 / (big_w * big_h) as f64 * 100.0);
    } else if args.len() > 1 && args[1] == "big" {
        let s: u32 = args.get(2).and_then(|x| x.parse().ok()).unwrap_or(0);
        let yy: u32 = args.get(3).and_then(|x| x.parse().ok()).unwrap_or(0);
        let t: u32 = args.get(4).and_then(|x| x.parse().ok()).unwrap_or(0);

        let big_w = 800i32;
        let big_h = 800i32;
        let mut cv = Cv { d: vec![BG; (big_w * big_h) as usize], w: big_w, h: big_h };

        let mx = ((s * 31 + yy * 17 + t * 7) % 10) as u32;
        let my = ((s * 13 + yy * 7 + t * 11) % 10) as u32;

        let m = 50i32;
        draw_container(&mut cv, m, m, big_w - 2 * m, big_h - 2 * m - 60, s, yy, t, mx, my);
        draw_label(&mut cv, m, big_h - 55, s, yy, t, mx, my);

        let out = format!("/mnt/data/catpaw/home/workspace/硅碳心源-果套循因/toolchain/tier1/mqf_glyph_big_s{}_y{}_t{}.bmp", s, yy, t);
        write_bmp_canvas(&cv, &out);

        let non_bg = cv.d.iter().filter(|&&c| c != BG).count();
        println!("放大: s={} y={} t={} mx={} my={} → {} 像素({:.1}%)",
                 s, yy, t, mx, my, out, non_bg as f64 / (big_w * big_h) as f64 * 100.0);
    } else if args.len() > 1 && args[1] == "tone" {
        // tone s y：固定声母+韵母，画全部声调变化（N×10 网格，N 从 toml 读）
        let s: u32 = args.get(2).and_then(|x| x.parse().ok()).unwrap_or(0);
        let yy: u32 = args.get(3).and_then(|x| x.parse().ok()).unwrap_or(0);

        let cell = 96i32;
        let cols = 10i32;
        // 行数从 toml 容量声明读取：行 = 声调进制 / 10
        let toml_path_tone = "/mnt/data/catpaw/home/workspace/硅碳心源-果套循因/toolchain/tier1/glyph_compiler/registry.toml";
        let cap_tone = registry::Capacity::from_toml(toml_path_tone);
        let rows = (cap_tone.tone_cap as i32 + 9) / 10;
        let label_left = 80i32;
        let title_top = 30i32;
        let gap = 2i32;
        let cw = cell - 2 * gap - 2;
        let ch = cell - 2 * gap - 2;

        let canvas_w = label_left + cols * cell + 10;
        let canvas_h = title_top + rows * cell + 10;

        let mut cv = Cv { d: vec![BG; (canvas_w * canvas_h) as usize], w: canvas_w, h: canvas_h };

        // 列号（顶部）
        for c in 0..cols {
            let px = label_left + c * cell + cell / 2 - 3;
            draw_tone(&mut cv, px, 5, 20, c as u32, c42_rgb(32));
        }
        // 行号（左侧）
        for r in 0..rows {
            let py = title_top + r * cell + cell / 2 - 3;
            draw_tone(&mut cv, 5, py, 60, (r * 10) as u32, c42_rgb(32));
        }

        let t_total = cap_tone.tone_cap;
        let mut drawn = 0u32;
        for t in 0..t_total {
            let col_num = (t % 10) as i32;
            let row_num = (t / 10) as i32;
            let ox = label_left + col_num * cell + gap + 1;
            let oy = title_top + row_num * cell + gap + 1;
            let mx = ((s * 31 + yy * 17 + t * 7) % 10) as u32;
            let my = ((s * 13 + yy * 7 + t * 11) % 10) as u32;
            draw_container(&mut cv, ox, oy, cw, ch, s, yy, t, mx, my);
            drawn += 1;
        }

        let out = format!("/mnt/data/catpaw/home/workspace/硅碳心源-果套循因/toolchain/tier1/mqf_glyph_tone_s{}_y{}.bmp", s, yy);
        write_bmp_canvas(&cv, &out);
        let non_bg = cv.d.iter().filter(|&&c| c != BG).count();
        println!("声调视图 s={} y={} t:0-{} 画布:{}x{} 像素:{}({:.1}%) → {}",
                 s, yy, t_total - 1, canvas_w, canvas_h, non_bg, non_bg as f64 / (canvas_w * canvas_h) as f64 * 100.0, out);
    } else if args.len() > 2 && args[1] == "pipe" {
        // pipeline：pipe s y t [S:±1 Y:-2 T: ...]一键生成标准 4 张 + 可选偏置对比
        let s: u32 = args.get(2).and_then(|x| x.parse().ok()).unwrap_or(0);
        let y: u32 = args.get(3).and_then(|x| x.parse().ok()).unwrap_or(0);
        let t: u32 = args.get(4).and_then(|x| x.parse().ok()).unwrap_or(0);
        let prefix = format!("/mnt/data/catpaw/home/workspace/硅碳心源-果套循因/toolchain/tier1/mqf_pipe_s{}_y{}_t{}", s, y, t);

        draw_pipe_one(s, y, t, &prefix);
        draw_pipe_tones(s, y, &prefix);
        draw_pipe_row(s, &prefix);
        draw_pipe_bias(s, y, t, &prefix);

        // 解析可选偏置参数（第 5 参数起），如果有，额外输出一张"偏置对比图"
        if args.len() > 5 {
            let mut bs_s: Vec<Bias> = Vec::new();
            let mut bs_y: Vec<Bias> = Vec::new();
            let mut bs_t: Vec<Bias> = Vec::new();
            for i in 5..args.len() {
                let a = &args[i];
                let (pf, nota) = if a.contains(':') {
                    let p: Vec<&str> = a.splitn(2, ':').collect();
                    (p[0], p[1])
                } else { ("*", a.as_str()) };
                let decoded = decode(nota);
                match pf {
                    "S" => bs_s.extend(decoded),
                    "Y" => bs_y.extend(decoded),
                    "T" => bs_t.extend(decoded),
                    _ => { bs_s.extend(decoded.clone()); bs_y.extend(decoded.clone()); bs_t.extend(decoded); }
                }
            }
            if !bs_s.is_empty() || !bs_y.is_empty() || !bs_t.is_empty() {
                let big_w = 800i32; let big_h = 800i32; let m = 60;
                const BG_DARK: u32 = 1118481;
                let mut cv = Cv { d: vec![BG_DARK; (big_w * big_h) as usize], w: big_w, h: big_h };
                draw_container_biased(&mut cv, m, m, big_w - 2*m, big_h - 2*m, s, y, t, &bs_s, &bs_y, &bs_t, c42_rgb(0));
                let out = format!("{}_biased.bmp", prefix);
                write_bmp_canvas(&cv, &out);
                println!("  ✓ 偏置对比 → {}", out);
            }
        }

        println!("pipeline done: s={} y={} t={} → {}_*.bmp", s, y, t, prefix);
    } else {
        // ─── 默认：8×8 索引图（64 个音节，每个 80×80 看清细节）───
        let cell = 80i32;
        let cols = 8i32;
        let rows = 8i32;
        let label_left = 70i32;
        let title_top = 30i32;

        let cw = cell - 4;
        let ch = cell - 4;
        let canvas_w = label_left + cols * cell + 8;
        let canvas_h = title_top + rows * cell + 8;
        let mut cv = Cv { d: vec![BG; (canvas_w * canvas_h) as usize], w: canvas_w, h: canvas_h };

        for c in 0..cols {
            let px = label_left + c * cell + cell/2 - 12;
            draw_yunmu(&mut cv, px, 5, 24, c as u32, c42_rgb(24));
            for dx in 0..(c as i32) {
                put(&mut cv, px + 15 + dx*2, 10, c42_rgb(32));
            }
        }
        for r in 0..rows {
            let py = title_top + r * cell + cell/2 - 8;
            draw_shengmu(&mut cv, 8, py, 24, r as u32, c42_rgb(16));
        }

        let tone = 5u32;
        let mut drawn = 0u32;
        for s in 0..rows as u32 {
            for y in 0..cols as u32 {
                let ox = label_left + y as i32 * cell + 2;
                let oy = title_top + s as i32 * cell + 2;
                let mx = ((s * 31 + y * 17 + tone * 7) % 10) as u32;
                let my = ((s * 13 + y * 7 + tone * 11) % 10) as u32;
                draw_container(&mut cv, ox, oy, cw, ch, s, y, tone, mx, my);
                drawn += 1;
            }
        }

        let out = "/mnt/data/catpaw/home/workspace/硅碳心源-果套循因/toolchain/tier1/mqf_glyph_v0_3.bmp";
        write_bmp_canvas(&cv, out);
        let non_bg = cv.d.iter().filter(|&&c| c != BG).count();
        println!("索引图 声母:{}×韵母:{}={}音节 @ 画布:{}x{} 像素:{}({:.1}%) → {}",
                 rows, cols, drawn, canvas_w, canvas_h,
                 non_bg, non_bg as f64 / (canvas_w * canvas_h) as f64 * 100.0, out);
    }
}

// ─── Pipeline 子函数 ─────────────────────────────────────

fn draw_pipe_one(s: u32, y: u32, t: u32, prefix: &str) {
    let big_w = 800i32;
    let big_h = 800i32;
    let mut cv = Cv { d: vec![c42_rgb(0); (big_w * big_h) as usize], w: big_w, h: big_h };
    let m = 40i32;
    draw_container_mono(&mut cv, m, m, big_w - 2*m, big_h - 2*m, s, y, t,
                        ((s*31+y*17+t*7)%10) as u32,
                        ((s*13+y*7+t*11)%10) as u32, c42_rgb(41));
    let out = format!("{}_one.bmp", prefix);
    write_bmp_canvas(&cv, &out);
    println!("  ✓ 单音节 mono → {}", out);
}

fn draw_pipe_tones(s: u32, y: u32, prefix: &str) {
    // 容量从 toml 读，不自创
    let toml_path = "/mnt/data/catpaw/home/workspace/硅碳心源-果套循因/toolchain/tier1/glyph_compiler/registry.toml";
    let cap = registry::Capacity::from_toml(toml_path);
    let t_cap = cap.tone_cap;

    let cell = 96i32;
    let cols = 10i32;
    let rows = (t_cap as i32 + 9) / 10;  // 7行 × 10列 = 70
    let label_left = 80i32;
    let title_top = 30i32;
    let cw = cell - 6;
    let canvas_w = label_left + cols * cell + 10;
    let canvas_h = title_top + rows * cell + 10;
    let mut cv = Cv { d: vec![BG; (canvas_w * canvas_h) as usize], w: canvas_w, h: canvas_h };
    for c in 0..cols {
        draw_tone(&mut cv, label_left + c * cell + cell/2 - 10, 5, 20, c as u32, c42_rgb(32));
    }
    for r in 0..rows {
        draw_tone(&mut cv, 5, title_top + r * cell + cell/2 - 8, 60, (r*10) as u32, c42_rgb(32));
    }
    for t in 0..t_cap {
        let ox = label_left + (t as i32 % 10) * cell + 3;
        let oy = title_top + (t as i32 / 10) * cell + 3;
        draw_container(&mut cv, ox, oy, cw, cw, s, y, t,
                       ((s*31+y*17+t*7)%10) as u32,
                       ((s*13+y*7+t*11)%10) as u32);
    }
    let out = format!("{}_tones.bmp", prefix);
    write_bmp_canvas(&cv, &out);
    println!("  ✓ 声调视图 {} → {}", t_cap, out);
}

fn draw_pipe_row(s: u32, prefix: &str) {
    draw_row(s, &format!("{}_row.bmp", prefix));
}

fn draw_pipe_bias(s: u32, y: u32, t: u32, prefix: &str) {
    let big_w = 800i32;
    let big_h = 800i32;
    const BG_DARK: u32 = 1118481;
    let mut cv = Cv { d: vec![BG_DARK; (big_w * big_h) as usize], w: big_w, h: big_h };
    let m = 60i32;
    let cont_w = big_w - 2 * m;
    let cont_h = big_h - 2 * m - 80;
    draw_container(&mut cv, m, m, cont_w, cont_h, s, y, t,
                   ((s*31+y*17+t*7)%10) as u32,
                   ((s*13+y*7+t*11)%10) as u32);
    let cx = m + cont_w / 2;
    let cy = m + cont_h / 2;
    let examples = vec![
        ("+1", c42_rgb(3), vec![(1i32, 0i32, 8i32, 0i32)]),
        ("±2", c42_rgb(10), vec![(2, 0, cont_w/6, 0)]),
        ("-1.2+", c42_rgb(17), vec![(1, 0, -8, 0), (2, 0, cont_w/6, 0)]),
        ("±3", c42_rgb(24), vec![(3, 1, 0, cont_h/6)]),
    ];
    for (_name, col, steps) in &examples {
        let (mut x, mut y_pos) = (cx, cy);
        for &(mag, _dir, dx, dy) in steps {
            let nx = x + dx;
            let ny = y_pos + dy;
            thick_line(&mut cv, x, y_pos, nx, ny, *col, 3);
            rect(&mut cv, nx-3, ny-3, nx+3, ny+3, *col);
            x = nx;
            y_pos = ny;
        }
    }
    let out = format!("{}_bias.bmp", prefix);
    write_bmp_canvas(&cv, &out);
    println!("  ✓ 偏置演示 → {}", out);
}
