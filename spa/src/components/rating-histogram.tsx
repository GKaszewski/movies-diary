type RatingHistogramProps = {
  histogram: number[]
}

export function RatingHistogram({ histogram }: RatingHistogramProps) {
  const max = Math.max(...histogram, 1)

  return (
    <div>
      <div className="flex items-end gap-1" style={{ height: 40 }}>
        {histogram.map((count, i) => (
          <div
            key={i}
            className="flex-1 rounded-t bg-amber-500/80"
            style={{ height: `${(count / max) * 100}%`, minHeight: count > 0 ? 2 : 0 }}
          />
        ))}
      </div>
      <div className="mt-1 flex gap-1">
        {[1, 2, 3, 4, 5].map((n) => (
          <div key={n} className="flex-1 text-center text-[10px] text-muted-foreground/40">{n}</div>
        ))}
      </div>
    </div>
  )
}
