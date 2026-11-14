import * as React from "react";

import { cn } from "@/lib/utils";

/**
 * Surfaces follow one light source (upper-left). `raise` is a panel sitting on
 * the substrate, `well` is a display recessed into it, and `float` is the only
 * translucent one — reserved for transient overlays, per Fluent's material
 * rules. Omitted means the flat border-only surface used outside the dashboard.
 *
 * The actual paint lives in `globals.css` as attribute selectors rather than
 * Tailwind classes: the bevel tokens are mode-dependent and no utility refers
 * to them, so putting them in `@theme` would get them pruned from the build.
 */
export type CardElevation = "raise" | "well" | "float";

function Card({
  className,
  elevation,
  ...props
}: React.ComponentProps<"div"> & { elevation?: CardElevation }) {
  return (
    <div
      data-slot="card"
      data-elevation={elevation}
      className={cn(
        "flex flex-col gap-4 rounded-xl border bg-card py-4 text-card-foreground",
        className,
      )}
      {...props}
    />
  );
}

function CardHeader({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="card-header"
      className={cn(
        "@container/card-header grid auto-rows-min grid-rows-[auto_auto] items-start gap-2 px-4 has-data-[slot=card-action]:grid-cols-[1fr_auto] [.border-b]:pb-3",
        className,
      )}
      {...props}
    />
  );
}

function CardTitle({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="card-title"
      className={cn("leading-none font-medium", className)}
      {...props}
    />
  );
}

function CardDescription({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="card-description"
      className={cn("text-sm text-muted-foreground", className)}
      {...props}
    />
  );
}

function CardAction({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="card-action"
      className={cn(
        "col-start-2 row-span-2 row-start-1 self-start justify-self-end",
        className,
      )}
      {...props}
    />
  );
}

function CardContent({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="card-content"
      className={cn("px-4", className)}
      {...props}
    />
  );
}

function CardFooter({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="card-footer"
      className={cn("flex items-center px-4 [.border-t]:pt-3", className)}
      {...props}
    />
  );
}

export {
  Card,
  CardHeader,
  CardFooter,
  CardTitle,
  CardAction,
  CardDescription,
  CardContent,
};
