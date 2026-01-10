interface ChartProps {
    data: string | number[];
    title?: string;
    type?: "bar" | "line";
    color?: string;
}

export default function Chart({ data: dataProp, title, type = "bar", color }: ChartProps) {
    // Parse data - can be comma-separated string or array
    const data: number[] = typeof dataProp === "string"
        ? dataProp.split(",").map((s) => parseFloat(s.trim())).filter((n) => !isNaN(n))
        : dataProp;

    if (!data || data.length === 0) {
        return <div className="chart chart--empty">No data available</div>;
    }

    const max = Math.max(...data);
    const min = Math.min(...data);
    const range = max - min || 1;
    const barWidth = 100 / data.length;
    const chartColor = color || "var(--color-primary, #3b82f6)";

    return (
        <div className="chart">
            {title && <h3 className="chart__title">{title}</h3>}
            <svg viewBox="0 0 100 50" className="chart__svg" preserveAspectRatio="none">
                {type === "bar" ? (
                    data.map((value, i) => {
                        const height = ((value - min) / range) * 45 + 5;
                        return (
                            <rect
                                key={i}
                                x={i * barWidth + barWidth * 0.1}
                                y={50 - height}
                                width={barWidth * 0.8}
                                height={height}
                                fill={chartColor}
                                rx="1"
                            />
                        );
                    })
                ) : (
                    <polyline
                        fill="none"
                        stroke={chartColor}
                        strokeWidth="1"
                        points={data
                            .map((value, i) => {
                                const x = (i / (data.length - 1)) * 100;
                                const y = 50 - ((value - min) / range) * 45 - 2.5;
                                return `${x},${y}`;
                            })
                            .join(" ")}
                    />
                )}
            </svg>
            <div className="chart__labels">
                <span className="chart__label chart__label--min">{min}</span>
                <span className="chart__label chart__label--max">{max}</span>
            </div>
        </div>
    );
}
