routine fibonacci(n : integer) : integer is
  if n <= 2 then 
    return 1;
  else
    return fibonacci(n  - 1) + fibonacci(n  - 2); 
  end
end

routine main() is 
  print fibonacci(100);
end
