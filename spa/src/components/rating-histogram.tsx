import { Bar, BarChart, XAxis } from "recharts"
import { ChartContainer, ChartTooltip, ChartTooltipContent, type ChartConfig } from "@/components/ui/chart"

const chartConfig = {
  count: { label: "Reviews", color: "var(--primary)" },
} satisfies ChartConfig

type RatingHistogramProps = {
  histogram: number[]
}

export function RatingHistogram({ histogram }: RatingHistogramProps) {
  const data = histogram.map((count, i) => ({ rating: `${i + 1}★`, count }))

  return (
    <ChartContainer config={chartConfig} className="aspect-[3/1] w-full">
      <BarChart data={data} margin={{ top: 4, right: 0, bottom: 0, left: 0 }}>
        <XAxis dataKey="rating" tick={{ fontSize: 11, fill: "rgba(255,255,255,0.85)" }} tickLine={false} axisLine={false} />
        <ChartTooltip content={<ChartTooltipContent hideIndicator />} />
        <Bar dataKey="count" fill="var(--color-count)" radius={[4, 4, 0, 0]} />
      </BarChart>
    </ChartContainer>
  )
}
