/** Control deslizante con etiqueta y valor. Doble clic vuelve al valor por defecto. */
export function Slider({
  label,
  min,
  max,
  step,
  value,
  onChange,
  format = (v) => String(v),
  defaultValue,
}: {
  label: string;
  min: number;
  max: number;
  step: number;
  value: number;
  onChange: (v: number) => void;
  format?: (v: number) => string;
  defaultValue?: number;
}) {
  return (
    <label className="slider" title={defaultValue != null ? "Doble clic para restablecer" : undefined}>
      <span>{label}</span>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        onDoubleClick={() => defaultValue != null && onChange(defaultValue)}
      />
      <span className="slider-value">{format(value)}</span>
    </label>
  );
}
