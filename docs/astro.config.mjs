import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

export default defineConfig({
  site: "https://brianp.github.io",
  base: "/spectacular",
  integrations: [
    starlight({
      title: "Spectacular",
      logo: {
        src: "./src/assets/logo.png",
        alt: "Spectacular",
      },
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/brianp/spectacular",
        },
      ],
      customCss: ["./src/styles/custom.css"],
      sidebar: [
        {
          label: "Getting Started",
          autogenerate: { directory: "getting-started" },
        },
        {
          label: "Guides",
          autogenerate: { directory: "guides" },
        },
        {
          label: "Reference",
          autogenerate: { directory: "reference" },
        },
      ],
    }),
  ],
});
