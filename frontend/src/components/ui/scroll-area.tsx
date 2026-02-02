import { ScrollArea as ScrollAreaPrimitive } from "radix-ui";
import type * as React from "react";

import { cn } from "@/lib/utils";

/**
 * Wraps Radix ScrollArea Root to provide a styled scrollable viewport with a scrollbar and corner.
 *
 * Applies base layout classes to the root, renders children inside the viewport, and forwards any additional props to the underlying Radix Root.
 *
 * @param className - Optional CSS class(es) to apply to the root container
 * @param children - Content rendered inside the scroll viewport
 * @returns The ScrollArea element containing a viewport, scrollbar, and corner
 */
function ScrollArea({
  className,
  children,
  ...props
}: React.ComponentProps<typeof ScrollAreaPrimitive.Root>) {
  return (
    <ScrollAreaPrimitive.Root
      data-slot="scroll-area"
      className={cn("relative overflow-hidden", className)}
      {...props}
    >
      <ScrollAreaPrimitive.Viewport
        data-slot="scroll-area-viewport"
        className="h-full w-full rounded-[inherit]"
      >
        {children}
      </ScrollAreaPrimitive.Viewport>
      <ScrollBar />
      <ScrollAreaPrimitive.Corner />
    </ScrollAreaPrimitive.Root>
  );
}

/**
 * Render a scrollbar for a ScrollArea with orientation-aware styling.
 *
 * Applies base and orientation-specific classes and renders a thumb element.
 *
 * @param className - Additional CSS class names to append to the scrollbar
 * @param orientation - Scrollbar orientation, either `"vertical"` or `"horizontal"`. Defaults to `"vertical"`.
 * @param props - Additional props forwarded to the underlying ScrollAreaScrollbar
 * @returns A ScrollAreaScrollbar React element with a styled thumb
 */
function ScrollBar({
  className,
  orientation = "vertical",
  ...props
}: React.ComponentProps<typeof ScrollAreaPrimitive.ScrollAreaScrollbar>) {
  return (
    <ScrollAreaPrimitive.ScrollAreaScrollbar
      data-slot="scroll-area-scrollbar"
      orientation={orientation}
      className={cn(
        "flex touch-none p-px transition-colors select-none",
        orientation === "vertical" && "h-full w-2.5 border-l border-l-transparent",
        orientation === "horizontal" && "h-2.5 flex-col border-t border-t-transparent",
        className,
      )}
      {...props}
    >
      <ScrollAreaPrimitive.ScrollAreaThumb
        data-slot="scroll-area-thumb"
        className="bg-border relative flex-1 rounded-full"
      />
    </ScrollAreaPrimitive.ScrollAreaScrollbar>
  );
}

export { ScrollArea, ScrollBar };