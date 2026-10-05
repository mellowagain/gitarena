"use client";

import { useState } from "react";
import { format, parse } from "date-fns";
import { CalendarIcon } from "lucide-react";
import type { Matcher } from "react-day-picker";
import { Button } from "@/components/ui/button";
import { Calendar } from "@/components/ui/calendar";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { cn } from "@/lib/utils";

interface DatePickerProps {
    value: string;
    onChange: (value: string) => void;
    disabled?: Matcher | Matcher[];
    required?: boolean;
    className?: string;
}

export function DatePicker({ value, onChange, disabled, required, className }: DatePickerProps) {
    const [open, setOpen] = useState(false);
    const selected = value ? parse(value, "yyyy-MM-dd", new Date()) : undefined;

    return (
        <Popover open={open} onOpenChange={setOpen}>
            <PopoverTrigger asChild>
                <Button variant="outline" className={cn("justify-start font-normal", !selected && "text-muted-foreground", className)}>
                    <CalendarIcon />
                    {selected ? format(selected, "MMM d, yyyy") : "Pick a date"}
                </Button>
            </PopoverTrigger>
            <PopoverContent className="w-auto p-0" align="start">
                <Calendar
                    mode="single"
                    selected={selected}
                    defaultMonth={selected}
                    disabled={disabled}
                    required={required}
                    onSelect={(date: Date | undefined) => {
                        onChange(date ? format(date, "yyyy-MM-dd") : "");
                        setOpen(false);
                    }}
                />
            </PopoverContent>
        </Popover>
    );
}
