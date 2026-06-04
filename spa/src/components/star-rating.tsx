import { Star } from "lucide-react"
import { cn } from "@/lib/utils"
import { hapticLight } from "@/lib/haptics"

type StarRatingProps = {
  value: number
  onChange: (value: number) => void
  size?: "sm" | "md" | "lg"
}

const sizes = { sm: "size-5", md: "size-8", lg: "size-10" }

export function StarRating({ value, onChange, size = "lg" }: StarRatingProps) {
  return (
    <div className="flex gap-1">
      {[1, 2, 3, 4, 5].map((star) => (
        <button
          key={star}
          type="button"
          onClick={() => { hapticLight(); onChange(star) }}
          className="transition-transform active:scale-90"
        >
          <Star
            className={cn(
              sizes[size],
              star <= value
                ? "fill-amber-500 text-amber-500 aero-star-filled"
                : "text-muted-foreground/30",
            )}
          />
        </button>
      ))}
    </div>
  )
}
