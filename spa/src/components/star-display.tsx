import { Star } from "lucide-react"
import { cn } from "@/lib/utils"

type StarDisplayProps = {
  rating: number
  size?: "xs" | "sm" | "md"
}

const sizes = { xs: "size-3", sm: "size-3.5", md: "size-4" }

export function StarDisplay({ rating, size = "sm" }: StarDisplayProps) {
  return (
    <div className="flex">
      {[1, 2, 3, 4, 5].map((star) => (
        <Star
          key={star}
          className={cn(
            sizes[size],
            star <= rating
              ? "fill-amber-500 text-amber-500 aero-star-filled"
              : "text-muted-foreground/20",
          )}
        />
      ))}
    </div>
  )
}
