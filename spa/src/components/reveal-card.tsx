import { useScrollReveal } from "@/hooks/use-animate"

export function RevealCard({ children, delay = 0 }: { children: React.ReactNode; delay?: number }) {
  const { ref, visible } = useScrollReveal()
  return (
    <div
      ref={ref}
      className="transition-all duration-700 ease-out"
      style={{
        opacity: visible ? 1 : 0,
        transform: visible ? "translateY(0) scale(1)" : "translateY(24px) scale(0.97)",
        transitionDelay: visible ? `${delay}ms` : "0ms",
      }}
    >
      {children}
    </div>
  )
}
