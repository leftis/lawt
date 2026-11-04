interface Props {
  /** Width and height in CSS pixels. */
  size?: number;
}

const OUTER = 200;
const INNER = 92;
const HOLE = 46;
const SHADES = ["#e94560", "#f26b82", "#d63a55", "#f05a73", "#c9304c", "#f7899b"] as const;

/** Point at `radius` and `degrees` clockwise from the top, around the centre of the 512 px canvas. */
function point(radius: number, degrees: number): string {
  const a = ((degrees - 90) * Math.PI) / 180;
  return `${(256 + radius * Math.cos(a)).toFixed(1)},${(256 + radius * Math.sin(a)).toFixed(1)}`;
}

/** The lawt mark: six aperture blades. Decorative, so hidden from screen readers. */
export function Logo({ size = 28 }: Props) {
  return (
    <svg width={size} height={size} viewBox="0 0 512 512" aria-hidden="true" focusable="false">
      {SHADES.map((fill, k) => {
        const a = 60 * k;
        const points = [point(OUTER, a), point(OUTER, a + 60), point(INNER, a + 98), point(HOLE, a + 24)].join(" ");
        return <polygon key={fill} points={points} fill={fill} stroke="#14141c" strokeWidth={6} strokeLinejoin="round" />;
      })}
    </svg>
  );
}
