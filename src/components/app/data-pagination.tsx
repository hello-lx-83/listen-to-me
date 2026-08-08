import {
  Pagination,
  PaginationContent,
  PaginationEllipsis,
  PaginationItem,
  PaginationLink,
  PaginationNext,
  PaginationPrevious,
} from "@/components/ui/pagination";

interface DataPaginationProps {
  page: number;
  pageCount: number;
  total: number;
  pageSize: number;
  onPageChange: (page: number) => void;
}

export function DataPagination({ page, pageCount, total, pageSize, onPageChange }: DataPaginationProps) {
  if (pageCount <= 1) return null;

  const start = (page - 1) * pageSize + 1;
  const end = Math.min(page * pageSize, total);
  const pages = visiblePages(page, pageCount);

  function goTo(nextPage: number) {
    onPageChange(Math.min(Math.max(nextPage, 1), pageCount));
  }

  return (
    <div className="flex flex-col items-center justify-between gap-3 border-t px-4 py-3 sm:flex-row">
      <p className="text-xs text-muted-foreground">显示 {start}–{end} 条，共 {total} 条</p>
      <Pagination className="mx-0 w-auto justify-end">
        <PaginationContent>
          <PaginationItem>
            <PaginationPrevious
              href="#"
              text="上一页"
              aria-disabled={page === 1}
              tabIndex={page === 1 ? -1 : undefined}
              className={page === 1 ? "pointer-events-none opacity-50" : undefined}
              onClick={(event) => { event.preventDefault(); goTo(page - 1); }}
            />
          </PaginationItem>
          {pages.map((item, index) => item === "ellipsis" ? (
            <PaginationItem key={`ellipsis-${index}`}><PaginationEllipsis /></PaginationItem>
          ) : (
            <PaginationItem key={item}>
              <PaginationLink
                href="#"
                isActive={item === page}
                aria-label={`第 ${item} 页`}
                onClick={(event) => { event.preventDefault(); goTo(item); }}
              >
                {item}
              </PaginationLink>
            </PaginationItem>
          ))}
          <PaginationItem>
            <PaginationNext
              href="#"
              text="下一页"
              aria-disabled={page === pageCount}
              tabIndex={page === pageCount ? -1 : undefined}
              className={page === pageCount ? "pointer-events-none opacity-50" : undefined}
              onClick={(event) => { event.preventDefault(); goTo(page + 1); }}
            />
          </PaginationItem>
        </PaginationContent>
      </Pagination>
    </div>
  );
}

function visiblePages(current: number, total: number): Array<number | "ellipsis"> {
  if (total <= 7) return Array.from({ length: total }, (_, index) => index + 1);
  if (current <= 4) return [1, 2, 3, 4, 5, "ellipsis", total];
  if (current >= total - 3) return [1, "ellipsis", total - 4, total - 3, total - 2, total - 1, total];
  return [1, "ellipsis", current - 1, current, current + 1, "ellipsis", total];
}
