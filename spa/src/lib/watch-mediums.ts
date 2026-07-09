import type { LucideIcon } from "lucide-react"
import {
  Clapperboard,
  Tv,
  Radio,
  Disc3,
  Download,
  Server,
  Ellipsis,
} from "lucide-react"

export type WatchMediumDef = {
  value: string
  icon: LucideIcon
  labelKey: string
}

export const WATCH_MEDIUMS: WatchMediumDef[] = [
  { value: "cinema", icon: Clapperboard, labelKey: "watchMedium.cinema" },
  { value: "streaming", icon: Tv, labelKey: "watchMedium.streaming" },
  { value: "tv", icon: Radio, labelKey: "watchMedium.tv" },
  { value: "physical_media", icon: Disc3, labelKey: "watchMedium.physicalMedia" },
  { value: "download", icon: Download, labelKey: "watchMedium.download" },
  { value: "media_server", icon: Server, labelKey: "watchMedium.mediaServer" },
  { value: "other", icon: Ellipsis, labelKey: "watchMedium.other" },
]
