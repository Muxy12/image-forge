import { createNavigation } from "next-intl/navigation";

import { routing } from "./routing";

/**
 * Locale-aware navigation helpers. `Link` and `getPathname` resolve the
 * localized pathname (including the `/convert` vs `/convertir` segment) for the
 * active or given locale.
 */
export const { Link, redirect, usePathname, useRouter, getPathname } =
  createNavigation(routing);
