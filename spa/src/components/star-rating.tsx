import { Star } from "lucide-react"
import { cn } from "@/lib/utils"
import { hapticLight } from "@/lib/haptics"

type StarRatingProps = {
  value: number
  onChange: (value: number) => void
  size?: "sm" | "md" | "lg"
}

const iconSizes = { sm: "size-5", md: "size-7", lg: "size-9" }
const buttonSizes = { sm: "size-8", md: "size-10", lg: "size-11" }

export function StarRating({ value, onChange, size = "lg" }: StarRatingProps) {
  return (
    <div className="flex gap-0.5">
      {[1, 2, 3, 4, 5].map((star) => (
        <button
          key={star}
          type="button"
          onClick={() => { hapticLight(); onChange(star) }}
          className={cn(
            "flex items-center justify-center rounded-md transition-transform active:scale-90",
            buttonSizes[size],
          )}
        >
          <Star
            className={cn(
              iconSizes[size],
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
