import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: {
    default: "ImageForge — Fast, private image conversion",
    template: "%s · ImageForge",
  },
  description:
    "Convert images in your browser or on the edge. SEO-first, developer-first.",
};

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
