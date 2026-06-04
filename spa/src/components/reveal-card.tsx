import { useScrollReveal } from "@/hooks/use-animate"

export function RevealCard({ children }: { children: React.ReactNode }) {
  const { ref, visible } = useScrollReveal()
  return (
    <div
      ref={ref}
      className="transition-all duration-700 ease-out"
      style={{
        opacity: visible ? 1 : 0,
        transform: visible ? "translateY(0)" : "translateY(24px)",
      }}
    >
      {children}
    </div>
  )
}
