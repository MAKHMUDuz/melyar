import { useRef, useEffect, useState, forwardRef, type RefObject, type ForwardedRef } from "react";
import { cn } from "@/lib/cn";
import { t, useLocale } from "@/lib/i18n";
import {
  surfaceActive,
  surfaceActiveTint,
  surfaceDisabled,
  surfaceLoading,
  surfacePrimaryGradient,
  surfaceTooltip,
} from "@/lib/surface";
import {
  prefersReducedMotion,
  getMotionTokens,
  applyFlipTransform,
  type FlipRect,
} from "@/lib/motion";

/* ============================================================
   SHARED GLASS MORPH INDICATOR
   ============================================================ */

interface GlassMorphIndicatorProps {
  targetRef: RefObject<HTMLElement | null>;
  containerRef: RefObject<HTMLElement | null>;
  isVisible?: boolean;
  /** Changes trigger a morph animation to the new target */
  targetKey?: string | number;
  variant?: "default" | "accent" | "subtle";
  radius?: "sm" | "md" | "lg" | "full";
  className?: string;
  duration?: number;
}

function GlassMorphIndicatorRender(
  props: GlassMorphIndicatorProps,
  ref: React.Ref<HTMLDivElement>
): React.ReactElement | null {
  const {
    targetRef,
    containerRef,
    isVisible = true,
    targetKey,
    variant = "default",
    radius = "md",
    className,
    duration,
  } = props;

  const indicatorRef = useRef<HTMLDivElement>(null);
  const lastRectRef = useRef<FlipRect | null>(null);
  const rafRef = useRef<number | undefined>(undefined);

  const variantClasses = {
    default: surfaceActiveTint,
    accent: "bg-accent/20 border-accent/40 shadow-[0_0_12px_rgba(250,35,59,0.3)]",
    subtle: "bg-white/5 border-white/10",
  };

  const radiusClasses = {
    sm: "rounded-sm",
    md: "rounded-md",
    lg: "rounded-lg",
    full: "rounded-full",
  };

  const updateIndicator = () => {
    const target = targetRef.current;
    const indicator = indicatorRef.current;
    if (!target || !indicator || !isVisible) return;

    const targetRect = target.getBoundingClientRect();
    const containerRect = containerRef.current?.getBoundingClientRect();

    if (containerRect) {
      indicator.style.left = `${targetRect.left - containerRect.left}px`;
      indicator.style.top = `${targetRect.top - containerRect.top}px`;
    } else {
      indicator.style.left = `${targetRect.left}px`;
      indicator.style.top = `${targetRect.top}px`;
    }

    indicator.style.width = `${targetRect.width}px`;
    indicator.style.height = `${targetRect.height}px`;
    indicator.style.borderRadius = radius === "full" ? "9999px" :
      radius === "sm" ? "6px" :
      radius === "lg" ? "16px" : "10px";

    lastRectRef.current = {
      x: targetRect.left,
      y: targetRect.top,
      width: targetRect.width,
      height: targetRect.height,
    };
  };

  const morphToTarget = () => {
    if (prefersReducedMotion()) {
      updateIndicator();
      return;
    }

    const target = targetRef.current;
    const indicator = indicatorRef.current;
    if (!target || !indicator) return;

    const first = lastRectRef.current || {
      x: parseFloat(indicator.style.left) || 0,
      y: parseFloat(indicator.style.top) || 0,
      width: indicator.offsetWidth || 0,
      height: indicator.offsetHeight || 0,
    };

    const targetRect = target.getBoundingClientRect();
    const containerRect = containerRef.current?.getBoundingClientRect();

    const last: FlipRect = containerRect
      ? {
          x: targetRect.left - containerRect.left,
          y: targetRect.top - containerRect.top,
          width: targetRect.width,
          height: targetRect.height,
        }
      : {
          x: targetRect.left,
          y: targetRect.top,
          width: targetRect.width,
          height: targetRect.height,
        };

    const cleanup = applyFlipTransform(indicator, first, last);
    lastRectRef.current = last;
    (indicator as any).__flipCleanup = cleanup;
  };

  useEffect(() => {
    if (!isVisible) return;

    updateIndicator();

    const handleResize = () => {
      if (rafRef.current) cancelAnimationFrame(rafRef.current);
      rafRef.current = requestAnimationFrame(() => {
        if (isVisible) morphToTarget();
      });
    };

    window.addEventListener("resize", handleResize);
    containerRef.current?.addEventListener("scroll", handleResize, { passive: true });

    return () => {
      window.removeEventListener("resize", handleResize);
      containerRef.current?.removeEventListener("scroll", handleResize);
      if (rafRef.current) cancelAnimationFrame(rafRef.current);
    };
  }, [isVisible, targetRef.current, containerRef.current]);

  useEffect(() => {
    if (isVisible && targetRef.current) {
      morphToTarget();
    }
  }, [targetRef.current, targetKey]);

  if (!isVisible) return null;

  return (
    <div
      ref={ref}
      className={cn(
        "pointer-events-none absolute transition-opacity",
        variantClasses[variant],
        radiusClasses[radius],
        "glass-interactive",
        className
      )}
      style={{
        opacity: isVisible ? 1 : 0,
        transformOrigin: "top left",
        willChange: "transform, width, height, left, top, border-radius",
      }}
      aria-hidden="true"
    />
  );
};

const GlassMorphIndicator = forwardRef<HTMLDivElement, GlassMorphIndicatorProps>(
  function(props: GlassMorphIndicatorProps, ref: React.Ref<HTMLDivElement>) {
    return GlassMorphIndicatorRender(props, ref);
  }
);
export { GlassMorphIndicator };

// Re-export the function for displayName
Object.assign(GlassMorphIndicator, { displayName: "GlassMorphIndicator" });

GlassMorphIndicator.displayName = "GlassMorphIndicator";

/* ============================================================
   GLASS SURFACE
   ============================================================ */

interface GlassSurfaceProps extends React.HTMLAttributes<HTMLDivElement> {
  intensity?: "base" | "elevated" | "strong" | "intense";
  interactive?: boolean;
  specular?: boolean;
  gradientBlur?: boolean;
  blur?: "xs" | "sm" | "md" | "lg" | "xl" | "2xl" | "3xl";
}

export const GlassSurface = forwardRef<HTMLDivElement, GlassSurfaceProps>(
  (props, ref) => {
    const {
      intensity = "elevated",
      interactive = false,
      specular = false,
      gradientBlur = false,
      blur,
      children,
      className,
      style,
      ...rest
    } = props;

    const intensityClasses = {
      base: "glass",
      elevated: "glass-elevated",
      strong: "glass-strong",
      intense: "glass-intense",
    };

    const blurClasses = {
      xs: "backdrop-blur-xs",
      sm: "backdrop-blur-sm",
      md: "backdrop-blur-md",
      lg: "backdrop-blur-lg",
      xl: "backdrop-blur-xl",
      "2xl": "backdrop-blur-2xl",
      "3xl": "backdrop-blur-3xl",
    };

    return (
      <div
        ref={ref}
        className={cn(
          intensityClasses[intensity],
          interactive && "glass-interactive",
          specular && "glass-specular",
          gradientBlur && "gradient-blur",
          blur && blurClasses[blur],
          className
        )}
        style={style}
        {...rest}
      >
        {children}
      </div>
    );
  }
);

GlassSurface.displayName = "GlassSurface";

/* ============================================================
   GLASS BUTTON
   ============================================================ */

interface GlassButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "default" | "icon" | "pill" | "primary" | "ghost";
  active?: boolean;
  loading?: boolean;
  size?: "sm" | "md" | "lg" | "xl";
  intensity?: "base" | "elevated" | "strong";
}

const sizeClasses = {
  sm: "h-8 px-3 text-xs",
  md: "h-10 px-4 text-sm",
  lg: "h-12 px-5 text-base",
  xl: "h-14 px-6 text-lg",
};

export const GlassButton = forwardRef<HTMLButtonElement, GlassButtonProps>(
  (props, ref) => {
    useLocale();
    const {
      variant = "default",
      active = false,
      loading = false,
      size = "md",
      intensity = "elevated",
      children,
      className,
      disabled,
      "aria-label": ariaLabel,
      ...rest
    } = props;

    const isIconOnly = variant === "icon";

    const baseClasses = cn(
      "inline-flex items-center justify-center gap-2 font-medium",
      "glass-interactive focus-ring",
      isIconOnly ? "rounded-full" : "rounded-full",
      sizeClasses[size],
      variant === "primary"
        ? cn(
            surfacePrimaryGradient,
            "shadow-[0_4px_16px_rgba(250,35,59,0.4),0_8px_32px_rgba(250,35,59,0.25)] hover:brightness-105 active:scale-[0.96]"
          )
        : variant === "ghost"
        ? "bg-transparent border-transparent hover:bg-glass-hover"
        : "glass-strong",
      active && variant !== "primary" && surfaceActive,
      loading && surfaceLoading,
      disabled && surfaceDisabled,
      className
    );

    const handleKeyDown = (e: React.KeyboardEvent<HTMLButtonElement>) => {
      if (e.key === " " || e.key === "Enter") {
        e.preventDefault();
        (e.currentTarget as HTMLButtonElement).click();
      }
    };

    return (
      <button
        ref={ref}
        className={baseClasses}
        disabled={disabled || loading}
        aria-label={ariaLabel}
        aria-pressed={variant !== "primary" && variant !== "ghost" ? active : undefined}
        onKeyDown={handleKeyDown}
        {...rest}
      >
        {loading ? (
          <span className="flex items-center gap-1.5">
            <svg
              className={cn("animate-spin", isIconOnly ? "size-4" : "size-5")}
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2.5"
              aria-hidden="true"
            >
              <circle cx="12" cy="12" r="10" strokeOpacity="0.25" />
              <path d="M12 2a10 10 0 0 1 10 10" strokeLinecap="round" strokeOpacity="1" />
            </svg>
            {!isIconOnly && <span>{t("common.loading")}</span>}
          </span>
        ) : (
          children
        )}
      </button>
    );
  }
);

GlassButton.displayName = "GlassButton";

/* ============================================================
   GLASS ICON BUTTON
   ============================================================ */

interface GlassIconButtonProps extends Omit<GlassButtonProps, "variant"> {
  tooltip?: string;
}

export const GlassIconButton = forwardRef<HTMLButtonElement, GlassIconButtonProps>(
  (props, ref) => {
    const {
      size = "md",
      intensity = "elevated",
      active = false,
      loading = false,
      tooltip,
      children,
      className,
      disabled,
      "aria-label": ariaLabel,
      ...rest
    } = props;

    const iconSizes = {
      sm: "size-8",
      md: "size-10",
      lg: "size-12",
      xl: "size-14",
    };

    const baseClasses = cn(
      "grid place-items-center rounded-full glass-interactive focus-ring",
      iconSizes[size],
      intensity === "strong" ? "glass-strong" : "glass-elevated",
      active && surfaceActive,
      loading && surfaceLoading,
      disabled && surfaceDisabled,
      className
    );

    return (
      <button
        ref={ref}
        className={baseClasses}
        disabled={disabled || loading}
        aria-label={ariaLabel || tooltip}
        {...rest}
      >
        {children}
        {tooltip && (
          <span
            className={cn(surfaceTooltip, "opacity-0 pointer-events-none transition-opacity group-hover:opacity-100")}
            role="tooltip"
          >
            {tooltip}
          </span>
        )}
      </button>
    );
  }
);

GlassIconButton.displayName = "GlassIconButton";

/* ============================================================
   GLASS TOGGLE SWITCH
   ============================================================ */

interface GlassToggleProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  "aria-label"?: string;
  size?: "sm" | "md" | "lg";
  className?: string;
}

const toggleTrackSizes = {
  sm: "w-8 h-5",
  md: "w-11 h-6",
  lg: "w-14 h-7",
};

const toggleThumbSizes = {
  sm: "size-4 translate-x-3",
  md: "size-5 translate-x-4",
  lg: "size-6 translate-x-5",
};

export const GlassToggle = forwardRef<HTMLButtonElement, GlassToggleProps>(
  (props, ref) => {
    const {
      checked,
      onChange,
      disabled = false,
      "aria-label": ariaLabel,
      size = "md",
      className,
    } = props;

    const thumbRef = useRef<HTMLDivElement>(null);
    const tokens = getMotionTokens();

    const handleClick = () => {
      if (!disabled) onChange(!checked);
    };

    const handleKeyDown = (e: React.KeyboardEvent<HTMLButtonElement>) => {
      if (e.key === " " || e.key === "Enter") {
        e.preventDefault();
        if (!disabled) onChange(!checked);
      }
    };

    return (
      <button
        ref={ref}
        role="switch"
        aria-checked={checked}
        aria-label={ariaLabel}
        disabled={disabled}
        onClick={handleClick}
        onKeyDown={handleKeyDown}
        className={cn(
          "relative inline-flex items-center",
          disabled && "opacity-40 pointer-events-none",
          className
        )}
        style={{ touchAction: "manipulation" }}
      >
        <div
          className={cn(
            "rounded-full glass-interactive transition-all",
            toggleTrackSizes[size],
            checked ? "bg-accent border-accent/30" : "bg-glass-border border-glass-border",
            "shadow-inner"
          )}
        >
          <div
            ref={thumbRef}
            className={cn(
              "rounded-full bg-white shadow-[0_2px_6px_rgba(0,0,0,0.4),0_0_0_2px_rgba(0,0,0,0.1)] transition-transform",
              toggleThumbSizes[size],
              checked && "translate-x-full",
              prefersReducedMotion() && "transition-none"
            )}
            style={{
              transition: prefersReducedMotion()
                ? "none"
                : `transform ${tokens.duration.toggle}ms ${tokens.easing.spring}`,
              willChange: "transform",
            }}
            aria-hidden="true"
          />
        </div>
      </button>
    );
  }
);

GlassToggle.displayName = "GlassToggle";

/* ============================================================
   GLASS SLIDER
   ============================================================ */

interface GlassSliderProps {
  min?: number;
  max?: number;
  step?: number;
  value?: number;
  showTooltip?: boolean;
  formatTooltip?: (value: number) => string;
  trackHeight?: number;
  thumbSize?: number;
  className?: string;
  style?: React.CSSProperties;
  onChange?: (value: number) => void;
  /** A vertical slider reads height as value, top being the maximum. Without a
   * real vertical mode the only way to stand a slider up was to rotate a
   * horizontal one, which maps the pointer across the rotated bounding box -
   * a few pixels of jitter sweeping the whole range. */
  orientation?: "horizontal" | "vertical";
  "aria-label"?: string;
}

export const GlassSlider = forwardRef<HTMLDivElement, GlassSliderProps>(
  (props, ref) => {
    const {
      min = 0,
      max = 1,
      step = 0.01,
      showTooltip = false,
      formatTooltip,
      orientation = "horizontal",
      trackHeight = 4,
      thumbSize = 16,
      className,
      style,
      onChange,
      value = 0,
      "aria-label": ariaLabel,
    } = props;

    const [isDragging, setIsDragging] = useState(false);
    const [isHovering, setIsHovering] = useState(false);
    const [hoverValue, setHoverValue] = useState<number | null>(null);
    const trackRef = useRef<HTMLDivElement>(null);
    const thumbRef = useRef<HTMLDivElement>(null);
    const tokens = getMotionTokens();

    const vertical = orientation === "vertical";
    const ratio = max === min ? 0 : (value - min) / (max - min);

    const localFraction = (clientX: number, clientY: number, rect: DOMRect) => {
      if (rect.width === 0 || rect.height === 0) return null;
      return vertical
        ? 1 - Math.max(0, Math.min(1, (clientY - rect.top) / rect.height))
        : Math.max(0, Math.min(1, (clientX - rect.left) / rect.width));
    };

    const getValueFromPoint = (clientX: number, clientY: number) => {
      const rect = trackRef.current?.getBoundingClientRect();
      if (!rect) return value;
      const local = localFraction(clientX, clientY, rect);
      if (local === null) return value;
      return min + local * (max - min);
    };

    const handlePointerDown = (e: React.PointerEvent<HTMLDivElement>) => {
      e.currentTarget.setPointerCapture(e.pointerId);
      setIsDragging(true);
      onChange?.(getValueFromPoint(e.clientX, e.clientY));
    };

    const handlePointerMove = (e: React.PointerEvent<HTMLDivElement>) => {
      if (!isDragging) return;
      onChange?.(getValueFromPoint(e.clientX, e.clientY));
    };

    const handlePointerUp = () => {
      setIsDragging(false);
    };

    const handleMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
      if (isDragging) return;
      const rect = trackRef.current?.getBoundingClientRect();
      if (!rect) return;
      const local = localFraction(e.clientX, e.clientY, rect);
      if (local === null) return;
      setHoverValue(min + local * (max - min));
    };

    const handleMouseLeave = () => {
      setHoverValue(null);
    };

    // The div is the slider and it has to answer the keyboard itself, because
    // the native input behind it is deliberately out of the tab order. Without
    // this, focus here handed every arrow key to the window-level shortcut,
    // which seeks and changes the volume while this thumb never moved - and
    // settings sliders could not be set from the keyboard at all.
    const handleKeyDown = (e: React.KeyboardEvent<HTMLDivElement>) => {
      let next: number | null = null;
      switch (e.key) {
        case "ArrowRight":
        case "ArrowUp":
          next = value + step;
          break;
        case "ArrowLeft":
        case "ArrowDown":
          next = value - step;
          break;
        case "PageUp":
          next = value + (max - min) / 10;
          break;
        case "PageDown":
          next = value - (max - min) / 10;
          break;
        case "Home":
          next = min;
          break;
        case "End":
          next = max;
          break;
        default:
          return;
      }
      e.preventDefault();
      // Stop the window-level shortcut from also seeking or changing volume.
      e.stopPropagation();
      onChange?.(Math.max(min, Math.min(max, next)));
    };

    return (
      <div
        ref={ref}
        className={cn(
          "relative cursor-pointer select-none",
          vertical ? "h-full" : "w-full",
          className,
        )}
        style={vertical ? { width: trackHeight, ...style } : { height: trackHeight, ...style }}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerLeave={handlePointerUp}
        onMouseMove={handleMouseMove}
        onMouseLeave={handleMouseLeave}
        onKeyDown={handleKeyDown}
        role="slider"
        aria-orientation={vertical ? "vertical" : "horizontal"}
        aria-valuemin={min}
        aria-valuemax={max}
        aria-valuenow={value}
        tabIndex={0}
        aria-label={ariaLabel}
      >
        <div
          className="absolute inset-0 rounded-full"
          style={{
            background: `color-mix(in oklab, var(--mute) 25%, transparent)`,
            borderRadius: "999px",
          }}
        />

        <div
          className="absolute rounded-full"
          style={
            vertical
              ? {
                  left: 0,
                  right: 0,
                  bottom: 0,
                  height: `${ratio * 100}%`,
                  background: "var(--accent)",
                  borderRadius: "999px",
                  boxShadow: "0 0 6px var(--accent), 0 0 12px var(--accent-soft)",
                  transition: `height ${tokens.duration.press}ms linear`,
                }
              : {
                  top: 0,
                  left: 0,
                  bottom: 0,
                  width: `${ratio * 100}%`,
                  background: "var(--accent)",
                  borderRadius: "999px",
                  boxShadow: "0 0 6px var(--accent), 0 0 12px var(--accent-soft)",
                  transition: `width ${tokens.duration.press}ms linear`,
                }
          }
        />

        <div
          ref={thumbRef}
          className="absolute pointer-events-none"
          style={
            vertical
              ? {
                  left: "50%",
                  bottom: `${ratio * 100}%`,
                  transform: `translate(-50%, 50%) scale(${isDragging ? 1.2 : isHovering ? 1.15 : 1})`,
                  transition: prefersReducedMotion()
                    ? "none"
                    : `transform ${tokens.duration.press}ms ${tokens.easing.spring}, bottom ${tokens.duration.press}ms linear`,
                  willChange: "transform, bottom",
                }
              : {
                  top: "50%",
                  left: `${ratio * 100}%`,
                  transform: `translate(-50%, -50%) scale(${isDragging ? 1.2 : isHovering ? 1.15 : 1})`,
                  transition: prefersReducedMotion()
                    ? "none"
                    : `transform ${tokens.duration.press}ms ${tokens.easing.spring}, left ${tokens.duration.press}ms linear`,
                  willChange: "transform, left",
                }
          }
        >
          <div
            className="rounded-full bg-white"
            style={{
              width: thumbSize,
              height: thumbSize,
              boxShadow:
                "0 2px 6px rgba(0,0,0,0.5), 0 0 0 2px var(--bg), 0 0 10px var(--accent-glow)",
            }}
          />
          {showTooltip && hoverValue !== null && (
            <div
              className={surfaceTooltip}
              role="tooltip"
            >
              {formatTooltip ? formatTooltip(hoverValue) : hoverValue.toFixed(0)}
            </div>
          )}
        </div>

        <input
          type="range"
          min={min}
          max={max}
          step={step}
          value={value}
          onChange={(e) => onChange?.(Number(e.target.value))}
          className="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
          style={{ appearance: "none", background: "transparent" }}
          aria-hidden="true"
          tabIndex={-1}
        />
      </div>
    );
  }
);

GlassSlider.displayName = "GlassSlider";

/* ============================================================
   GLASS SEGMENTED CONTROL
   ============================================================ */

interface GlassSegmentedProps {
  options: Array<{ value: string; label: string; icon?: React.ReactNode }>;
  value: string;
  onChange: (value: string) => void;
  className?: string;
  "aria-label"?: string;
}

export const GlassSegmented = forwardRef<HTMLDivElement, GlassSegmentedProps>(
  (props, ref) => {
    const { options, value, onChange, className, "aria-label": ariaLabel } = props;
    const containerRef = useRef<HTMLDivElement>(null);
    const optionRefs = useRef<Map<string, HTMLButtonElement>>(new Map());
    const activeOptionRef = useRef<HTMLButtonElement | null>(null);
    const tokens = getMotionTokens();

    const handleSelect = (optValue: string) => {
      onChange(optValue);
    };

    return (
      <div
        ref={ref}
        role="tablist"
        aria-label={ariaLabel}
        className={cn(
          "inline-flex items-center gap-1 p-1 glass-strong rounded-full",
          className
        )}
      >
        <GlassMorphIndicator
          targetRef={activeOptionRef}
          containerRef={containerRef}
          variant="accent"
          radius="full"
        />

        {options.map((option) => (
          <button
            key={option.value}
            ref={(el) => {
              optionRefs.current.set(option.value, el!);
              if (value === option.value) activeOptionRef.current = el;
            }}
            role="tab"
            aria-selected={value === option.value}
            aria-label={option.label}
            onClick={() => handleSelect(option.value)}
            className={cn(
              "relative z-10 flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium rounded-full",
              "transition-colors",
              value === option.value
                ? "text-accent-text"
                : "text-text-secondary hover:text-text",
              "focus-ring"
            )}
          >
            {option.icon && <span className="size-4">{option.icon}</span>}
            {option.label}
          </button>
        ))}
      </div>
    );
  }
);

GlassSegmented.displayName = "GlassSegmented";

/* ============================================================
   GLASS SEARCH
   ============================================================ */

interface GlassSearchProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, "type"> {
  placeholder?: string;
  onSearch?: (query: string) => void;
  debounceMs?: number;
  className?: string;
  autoFocus?: boolean;
}

export const GlassSearch = forwardRef<HTMLInputElement, GlassSearchProps>(
  (props, ref) => {
    const {
      placeholder = t("common.search"),
      onSearch,
      debounceMs = 300,
      className,
      autoFocus,
      onChange,
      onFocus,
      onBlur,
      ...rest
    } = props;

    const [isFocused, setIsFocused] = useState(false);
    const [query, setQuery] = useState("");
    const containerRef = useRef<HTMLDivElement>(null);
    const inputRef = useRef<HTMLInputElement>(null);
    const timeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
    const tokens = getMotionTokens();

    const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
      const value = e.target.value;
      setQuery(value);
      onChange?.(e);
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
      timeoutRef.current = setTimeout(() => onSearch?.(value), debounceMs);
    };

    const handleFocus = (e: React.FocusEvent<HTMLInputElement>) => {
      setIsFocused(true);
      onFocus?.(e);
    };

    const handleBlur = (e: React.FocusEvent<HTMLInputElement>) => {
      setIsFocused(false);
      onBlur?.(e);
    };

    const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "Escape") {
        inputRef.current?.blur();
      }
    };

    return (
      <div
        ref={containerRef}
        className={cn(
          "relative flex items-center gap-2 group",
          "transition-all",
          className
        )}
        style={{
          transition: `width ${tokens.duration.search}ms ${tokens.easing.standard}`,
        }}
      >
        <label
          className={cn(
            "flex items-center gap-2 w-full glass-strong rounded-full px-4",
            "transition-all",
            isFocused && "border-glass-border-focus ring-2 ring-accent/20"
          )}
        >
          <svg
            className={cn(
              "size-5 flex-shrink-0 transition-colors",
              isFocused ? "text-accent-text" : "text-text-tertiary"
            )}
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            aria-hidden="true"
          >
            <circle cx="11" cy="11" r="8" />
            <path d="M21 21l-4.35-4.35" />
          </svg>
          <input
            ref={ref}
            type="search"
            value={query}
            onChange={handleChange}
            onFocus={handleFocus}
            onBlur={handleBlur}
            onKeyDown={handleKeyDown}
            placeholder={placeholder}
            className={cn(
              "w-full bg-transparent text-base outline-none placeholder:text-text-tertiary",
              "transition-colors"
            )}
            autoFocus={autoFocus}
            autoComplete="off"
            spellCheck={false}
            {...rest}
          />
        </label>

        {isFocused && query && (
          <button
            type="button"
            onClick={() => {
              setQuery("");
              handleChange({ target: { value: "" } } as any);
              inputRef.current?.focus();
            }}
            className={cn(
              "grid size-9 place-items-center rounded-full glass-interactive",
              "opacity-0 pointer-events-none group-focus-within:opacity-100 group-focus-within:pointer-events-auto",
              "transition-opacity"
            )}
            aria-label={t("common.clear")}
          >
            <svg className="size-4 text-text-tertiary" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        )}
      </div>
    );
  }
);

GlassSearch.displayName = "GlassSearch";

/* ============================================================
   GLASS PANEL
   ============================================================ */

interface GlassPanelProps extends React.HTMLAttributes<HTMLDivElement> {
  mode?: "docked" | "floating" | "modal";
  open: boolean;
  onClose?: () => void;
  title?: string;
  titleActions?: React.ReactNode;
  showClose?: boolean;
  instantClose?: boolean;
}

export const GlassPanel = forwardRef<HTMLDivElement, GlassPanelProps>(
  (props, ref) => {
    const {
      mode = "docked",
      open,
      onClose,
      title,
      titleActions,
      showClose = true,
      instantClose = false,
      children,
      className,
      style,
      ...rest
    } = props;

    const panelRef = useRef<HTMLDivElement>(null);
    const tokens = getMotionTokens();
    const reduced = prefersReducedMotion();

    useEffect(() => {
      if (!open) return;
      const handleKey = (e: KeyboardEvent) => {
        if (e.key === "Escape") onClose?.();
      };
      window.addEventListener("keydown", handleKey);
      return () => window.removeEventListener("keydown", handleKey);
    }, [open, onClose]);

    useEffect(() => {
      if (open && panelRef.current) {
        const focusable = panelRef.current.querySelector<HTMLElement>(
          'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
        );
        focusable?.focus();
      }
    }, [open]);

    if (!open && instantClose) return null;

    const modeClasses = {
      docked: "fixed right-0 top-[var(--bar-h)] bottom-0 border-l border-glass-border-strong",
      floating: "fixed right-4 top-1/2 -translate-y-1/2 border border-glass-border-strong rounded-2xl",
      modal: "fixed inset-0 flex items-center justify-center p-6",
    };

    const openTransform = {
      docked: "translate-x-0",
      floating: "translate-y-[-50%] translate-x-0",
      modal: "",
    };

    const closedTransform = {
      docked: "translate-x-full",
      floating: "translate-y-[-50%] translate-x-full",
      modal: "",
    };

    return (
      <div
        ref={ref}
        role={mode === "modal" ? "dialog" : "region"}
        aria-modal={mode === "modal"}
        aria-label={title}
        className={cn(
          "glass-intense flex flex-col z-50",
          modeClasses[mode],
          open ? openTransform[mode] : closedTransform[mode],
          className
        )}
        style={{
          ...style,
          width: mode === "docked" ? "var(--queue-w)" : mode === "floating" ? "var(--queue-w-narrow)" : "100%",
          maxWidth: mode === "floating" ? "calc(100vw - 32px)" : undefined,
          maxHeight: mode === "floating" ? "calc(100vh - 120px)" : undefined,
          transitionDuration: reduced ? "0ms" : `${tokens.duration.panel}ms`,
          transitionTimingFunction: tokens.easing.panel,
        }}
        {...rest}
      >
        {mode === "modal" && (
          <div
            className="absolute inset-0 bg-black/50 backdrop-blur-sm"
            onClick={onClose}
            aria-hidden="true"
          />
        )}

        <div className="relative flex flex-col flex-1 overflow-hidden">
          {(title || showClose) && (
            <header className="flex items-center justify-between px-4 py-3 border-b border-glass-border shrink-0">
              {title && (
                <h2 className="text-sm font-semibold tracking-tight">{title}</h2>
              )}
              <div className="flex items-center gap-2">
                {titleActions}
                {showClose && (
                  <button
                    onClick={onClose}
                    className="icon-btn"
                    aria-label={t("common.close")}
                  >
                    <svg className="size-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                      <line x1="18" y1="6" x2="6" y2="18" />
                      <line x1="6" y1="6" x2="18" y2="18" />
                    </svg>
                  </button>
                )}
              </div>
            </header>
          )}

          <div className="flex-1 min-h-0 overflow-y-auto">
            {children}
          </div>
        </div>
      </div>
    );
  }
);

GlassPanel.displayName = "GlassPanel";

/* ============================================================
   PLAY/PAUSE MORPH BUTTON
   ============================================================ */

interface PlayPauseMorphProps {
  playing: boolean;
  onToggle: () => void;
  size?: "main" | "expanded" | "compact";
  "aria-label"?: string;
  className?: string;
}

export const PlayPauseMorph = forwardRef<HTMLButtonElement, PlayPauseMorphProps>(
  (props, ref) => {
    useLocale();
    const {
      playing,
      onToggle,
      size = "main",
      "aria-label": ariaLabel,
      className,
    } = props;

    const [isPressed, setIsPressed] = useState(false);
    const tokens = getMotionTokens();
    const reduced = prefersReducedMotion();

    const sizeClasses = {
      main: "size-14",
      expanded: "size-18",
      compact: "size-10",
    };

    const iconSizes = {
      main: "size-6",
      expanded: "size-10",
      compact: "size-5",
    };

    const handleMouseDown = () => setIsPressed(true);
    const handleMouseUp = () => setIsPressed(false);
    const handleMouseLeave = () => setIsPressed(false);

    return (
      <button
        ref={ref}
        onClick={onToggle}
        onMouseDown={handleMouseDown}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseLeave}
        className={cn(
          "grid place-items-center rounded-full",
          "transition-all",
          sizeClasses[size],
          playing
            ? surfacePrimaryGradient
            : "glass-strong",
          "focus-ring",
          isPressed && "scale-[0.95]",
          className
        )}
        style={{
          boxShadow: playing
            ? "0 4px 16px var(--accent-glow), 0 8px 32px var(--accent-glow-strong), inset 0 1px 0 rgba(255,255,255,0.18)"
            : undefined,
          transition: reduced
            ? "none"
            : `transform ${tokens.duration.press}ms ${tokens.easing.spring}, box-shadow ${tokens.duration.toggle}ms ${tokens.easing.standard}, background ${tokens.duration.toggle}ms ${tokens.easing.morph}`,
          willChange: "transform, box-shadow, background",
        }}
        aria-label={ariaLabel || (playing ? t("toolbar.pause") : t("toolbar.play"))}
        aria-pressed={playing}
      >
        {playing ? (
          <svg
            className={cn("fill-current transition-transform", iconSizes[size])}
            viewBox="0 0 24 24"
            style={{ transformOrigin: "center" }}
          >
            <rect x="6" y="4" width="4" height="16" rx="1" />
            <rect x="14" y="4" width="4" height="16" rx="1" />
          </svg>
        ) : (
          <svg
            className={cn("fill-current ml-0.5 transition-transform", iconSizes[size])}
            viewBox="0 0 24 24"
            style={{ transformOrigin: "center" }}
          >
            <path d="M8 5v14l11-7z" />
          </svg>
        )}
      </button>
    );
  }
);

PlayPauseMorph.displayName = "PlayPauseMorph";

/* ============================================================
   EXPORTS
   ============================================================ */

export {
  prefersReducedMotion,
  getMotionTokens,
  applyFlipTransform,
  type FlipRect,
  type GlassMorphIndicatorProps,
  type GlassSurfaceProps,
  type GlassButtonProps,
  type GlassIconButtonProps,
  type GlassToggleProps,
  type GlassSliderProps,
  type GlassSegmentedProps,
  type GlassSearchProps,
  type GlassPanelProps,
  type PlayPauseMorphProps,
};